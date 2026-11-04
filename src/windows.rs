use crate::{Error, TrashContext, TrashItem, TrashItemMetadata, TrashItemSize};
use std::{
    borrow::Borrow,
    cell::RefCell,
    ffi::{c_void, OsStr, OsString},
    os::windows::{ffi::OsStrExt, prelude::*},
    path::{Component, Path, PathBuf, Prefix},
};
use windows::Win32::{
    Foundation::*, Storage::EnhancedStorage::*, System::Com::*, System::SystemServices::*, UI::Shell::*,
};
use windows::{
    core::{implement, ComObject, Interface, Ref, HRESULT, PCWSTR, PWSTR},
    Win32::System::Com::StructuredStorage::PropVariantToBSTR,
};

const SCID_ORIGINAL_LOCATION: PROPERTYKEY = PROPERTYKEY { fmtid: PSGUID_DISPLACED, pid: PID_DISPLACED_FROM };
const SCID_DATE_DELETED: PROPERTYKEY = PROPERTYKEY { fmtid: PSGUID_DISPLACED, pid: PID_DISPLACED_DATE };

impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Error {
        let code = err.code();
        let description = match copy_engine_reason(code) {
            Some(reason) => format!("windows error: {reason} ({:#010x})", code.0 as u32),
            None => format!("windows error: {err}"),
        };
        Error::Os { code: code.0, description }
    }
}

/// `FACILITY_SHELL`: the codes of the shell's copy engine (`sherrors.h`).
const FACILITY_SHELL: u32 = 0x27;

/// `COPYENGINE_E_USER_CANCELLED`, e.g. a declined UAC prompt.
const COPYENGINE_E_USER_CANCELLED: u32 = 0x8027_0000;

/// What a copy engine failure (`COPYENGINE_E_*`, `0x8027xxxx`) means, in the
/// words of `sherrors.h`.
///
/// These are the reasons `IFileOperation` reports per item. The system has no
/// message text for them: `FormatMessage` either finds none or one from
/// another facility (`0x80270002` comes out as a Network Access Protection
/// message), so they are spelled out here. `None` for anything that is not a
/// copy engine failure, which the system can describe itself.
fn copy_engine_reason(code: HRESULT) -> Option<&'static str> {
    let code = code.0 as u32;
    if code & 0x8000_0000 == 0 || (code >> 16) & 0x1FFF != FACILITY_SHELL {
        return None;
    }
    Some(match code {
        0x8027_0000 => "the operation was cancelled",
        0x8027_0001 => "the operation was cancelled",
        0x8027_0002 => "the operation requires elevation",
        0x8027_001B => "invalid source path",
        0x8027_001C => "invalid destination path",
        0x8027_001D => "the source path is too long",
        0x8027_001E => "the destination path would be too long",
        0x8027_001F => "the source is a root directory",
        0x8027_0021 => "access denied to the source",
        0x8027_0022 => "access denied to the destination",
        0x8027_0023 => "the source does not exist or is unavailable",
        0x8027_0024 => "the destination does not exist or is unavailable",
        0x8027_0025 => "the source is on a disconnected network location",
        0x8027_0026 => "the destination is on a disconnected network location",
        0x8027_0027 => "the source is in use by another process",
        0x8027_0028 => "the destination is in use by another process",
        0x8027_0029..=0x8027_002C => "the destination already exists",
        0x8027_0032 | 0x8027_0033 => "the destination is full",
        0x8027_0035 => "the source cannot be reached",
        0x8027_0036 => "the Recycle Bin is not available on this drive",
        0x8027_0037 => "the item is too large for the Recycle Bin",
        0x8027_0038 => "the item is too deep for the Recycle Bin",
        0x8027_003A => "the Recycle Bin could not be found or is unavailable",
        0x8027_003D => "the directory is not empty",
        0x8027_003F => "the item is read-only",
        0x8027_0043 => "the item cannot be moved out of the Recycle Bin",
        0x8027_0045 | 0x8027_0046 => "the storage provider of the item is unavailable",
        0x8027_0048 => "blocked by policy",
        _ => "the shell's file operation failed",
    })
}

/// `windows error: <reason> (0x…)` for an item the shell did not process.
fn describe_item_failure(path: &Path, code: HRESULT) -> String {
    let reason = match copy_engine_reason(code) {
        Some(reason) => reason.to_string(),
        None => windows::core::Error::from(code).message(),
    };
    format!("windows error: {}: {reason} ({:#010x})", path.display(), code.0 as u32)
}

/// Whether the shell asks for administrator rights before it trashes or
/// restores an item that needs them.
///
/// Set with [`TrashContextExtWindows::set_elevation_prompt`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ElevationPrompt {
    /// Never ask. An item only an administrator may move fails with the
    /// shell's reason, e.g. `0x80270021` ("access denied to the source").
    #[default]
    Never,
    /// Let the shell show a UAC prompt for the items that need it, owned by
    /// this window (an `HWND`, which may belong to another process).
    ///
    /// The items go through once without any user interface; only those the
    /// shell refused for want of rights go again, in one operation that is
    /// elevated as a whole (`FOFX_REQUIREELEVATION`), so one UAC prompt covers
    /// them. They end up in — and come back from — the Recycle Bin of the
    /// account that approves the prompt.
    ///
    /// That second operation cannot suppress the shell's error dialogs:
    /// `FOF_NOERRORUI` suppresses the UAC prompt as well. Should an item fail
    /// for another reason once elevated, the shell says so in a dialog owned
    /// by the window. Declining the prompt fails with `0x80270000`
    /// ("the operation was cancelled").
    ///
    /// Without an owner window Windows shows the prompt behind other windows,
    /// which is why there is no variant without one.
    OwnedBy(isize),
}

/// Windows-specific options of a [`TrashContext`].
pub trait TrashContextExtWindows {
    /// Lets the shell ask for elevation when an item needs it.
    /// [`ElevationPrompt::Never`] is the default.
    fn set_elevation_prompt(&mut self, prompt: ElevationPrompt);
    fn elevation_prompt(&self) -> ElevationPrompt;

    /// [`os_limited::restore_all`](crate::os_limited::restore_all) with this
    /// context's [`ElevationPrompt`]: restoring into a folder only an
    /// administrator may write to (`0x80270022`) asks for elevation too.
    fn restore_all<I>(&self, items: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = TrashItem>;
}

impl TrashContextExtWindows for TrashContext {
    fn set_elevation_prompt(&mut self, prompt: ElevationPrompt) {
        self.platform_specific.elevation_prompt = prompt;
    }
    fn elevation_prompt(&self) -> ElevationPrompt {
        self.platform_specific.elevation_prompt
    }
    fn restore_all<I>(&self, items: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = TrashItem>,
    {
        let items = crate::os_limited::without_twins(items.into_iter().collect())?;
        restore_all_with(items, self.platform_specific.elevation_prompt)
    }
}

/// Failures that administrator rights may lift: the copy engine's
/// `REQUIRES_ELEVATION` and access to source or destination denied, or a
/// plain `E_ACCESSDENIED`. Whether they really do only the shell knows — an
/// access list that denies everyone denies administrators too.
fn needs_elevation(code: HRESULT) -> bool {
    matches!(code.0 as u32, 0x8027_0002 | 0x8027_0021 | 0x8027_0022) || code == E_ACCESSDENIED
}

/// The form both a queued item's path and the path the sink reports for it
/// are compared in.
fn item_key(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_lowercase()
}

/// Records each item's result as `IFileOperation` reports it.
///
/// `PerformOperations` and `FinishOperations` report success when individual
/// items were skipped (with `FOF_NO_UI` a refused item is skipped, not
/// asked about); only `GetAnyOperationsAborted` says that something was, and
/// only the per-item callbacks say why. One item may report more than once:
/// with an elevation prompt it reports `REQUIRES_ELEVATION` before it
/// succeeds, so only its last result counts.
#[implement(IFileOperationProgressSink)]
struct ResultSink {
    results: RefCell<Vec<(PathBuf, HRESULT)>>,
}

impl ResultSink {
    fn record(&self, item: Ref<'_, IShellItem>, result: HRESULT) {
        let path = item
            .as_ref()
            .and_then(|item| unsafe { get_display_name(item, SIGDN_FILESYSPATH) }.ok())
            .map(PathBuf::from)
            .unwrap_or_default();
        self.results.borrow_mut().push((path, result));
    }
}

#[allow(non_snake_case)] // the method names are COM's
impl IFileOperationProgressSink_Impl for ResultSink_Impl {
    fn StartOperations(&self) -> windows::core::Result<()> {
        Ok(())
    }
    fn FinishOperations(&self, _result: HRESULT) -> windows::core::Result<()> {
        Ok(())
    }
    fn PreRenameItem(&self, _flags: u32, _item: Ref<'_, IShellItem>, _new_name: &PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }
    fn PostRenameItem(
        &self,
        _flags: u32,
        item: Ref<'_, IShellItem>,
        _new_name: &PCWSTR,
        result: HRESULT,
        _new_item: Ref<'_, IShellItem>,
    ) -> windows::core::Result<()> {
        self.record(item, result);
        Ok(())
    }
    fn PreMoveItem(
        &self,
        _flags: u32,
        _item: Ref<'_, IShellItem>,
        _destination: Ref<'_, IShellItem>,
        _new_name: &PCWSTR,
    ) -> windows::core::Result<()> {
        Ok(())
    }
    fn PostMoveItem(
        &self,
        _flags: u32,
        item: Ref<'_, IShellItem>,
        _destination: Ref<'_, IShellItem>,
        _new_name: &PCWSTR,
        result: HRESULT,
        _new_item: Ref<'_, IShellItem>,
    ) -> windows::core::Result<()> {
        self.record(item, result);
        Ok(())
    }
    fn PreCopyItem(
        &self,
        _flags: u32,
        _item: Ref<'_, IShellItem>,
        _destination: Ref<'_, IShellItem>,
        _new_name: &PCWSTR,
    ) -> windows::core::Result<()> {
        Ok(())
    }
    fn PostCopyItem(
        &self,
        _flags: u32,
        item: Ref<'_, IShellItem>,
        _destination: Ref<'_, IShellItem>,
        _new_name: &PCWSTR,
        result: HRESULT,
        _new_item: Ref<'_, IShellItem>,
    ) -> windows::core::Result<()> {
        self.record(item, result);
        Ok(())
    }
    fn PreDeleteItem(&self, _flags: u32, _item: Ref<'_, IShellItem>) -> windows::core::Result<()> {
        Ok(())
    }
    fn PostDeleteItem(
        &self,
        _flags: u32,
        item: Ref<'_, IShellItem>,
        result: HRESULT,
        _new_item: Ref<'_, IShellItem>,
    ) -> windows::core::Result<()> {
        self.record(item, result);
        Ok(())
    }
    fn PreNewItem(
        &self,
        _flags: u32,
        _destination: Ref<'_, IShellItem>,
        _new_name: &PCWSTR,
    ) -> windows::core::Result<()> {
        Ok(())
    }
    fn PostNewItem(
        &self,
        _flags: u32,
        _destination: Ref<'_, IShellItem>,
        _new_name: &PCWSTR,
        _template_name: &PCWSTR,
        _attributes: u32,
        _result: HRESULT,
        _new_item: Ref<'_, IShellItem>,
    ) -> windows::core::Result<()> {
        Ok(())
    }
    fn UpdateProgress(&self, _total: u32, _so_far: u32) -> windows::core::Result<()> {
        Ok(())
    }
    fn ResetTimer(&self) -> windows::core::Result<()> {
        Ok(())
    }
    fn PauseTimer(&self) -> windows::core::Result<()> {
        Ok(())
    }
    fn ResumeTimer(&self) -> windows::core::Result<()> {
        Ok(())
    }
}

/// What one `PerformOperations` reported.
struct Performed {
    /// The last result of each item, in the order the items first reported.
    results: Vec<(PathBuf, HRESULT)>,
    performed: windows::core::Result<()>,
    aborted: windows::core::Result<bool>,
    unadvised: windows::core::Result<()>,
}

impl Performed {
    fn failures(&self) -> Vec<(PathBuf, HRESULT)> {
        self.results.iter().filter(|(_, code)| code.is_err()).cloned().collect()
    }

    /// `Ok` when every item went through, else the first failure, naming the
    /// item and the reason.
    fn into_result(self) -> Result<(), Error> {
        let failures = self.failures();
        // A declined UAC prompt: the items report only that they needed
        // elevation, the operation as a whole that it was cancelled — which
        // is what happened.
        if let Err(error) = &self.performed {
            if error.code().0 as u32 == COPYENGINE_E_USER_CANCELLED {
                let path = failures.first().map(|(path, _)| path.clone()).unwrap_or_default();
                return Err(failure_error(&[(path, error.code())]));
            }
        }
        if !failures.is_empty() {
            return Err(failure_error(&failures));
        }
        self.performed?;
        if self.aborted? {
            // Aborted without a reason for any item, e.g. cancelled as a whole.
            return Err(Error::Unknown { description: "Some operations were aborted".into() });
        }
        self.unadvised?;
        Ok(())
    }
}

fn failure_error(failures: &[(PathBuf, HRESULT)]) -> Error {
    let Some((path, code)) = failures.first() else {
        return Error::Unknown { description: "Some operations were aborted".into() };
    };
    let mut description = describe_item_failure(path, *code);
    if failures.len() > 1 {
        description.push_str(&format!(" (and {} more item(s))", failures.len() - 1));
    }
    Error::Os { code: code.0, description }
}

/// Keeps the last result of each item, in the order of first appearance.
fn last_result_per_item(results: Vec<(PathBuf, HRESULT)>) -> Vec<(PathBuf, HRESULT)> {
    let mut last: Vec<(PathBuf, HRESULT)> = Vec::with_capacity(results.len());
    for (path, code) in results {
        match last.iter_mut().find(|(seen, _)| item_key(seen) == item_key(&path)) {
            Some(entry) => entry.1 = code,
            None => last.push((path, code)),
        }
    }
    last
}

/// Runs `operation`'s queued work with a [`ResultSink`] attached.
unsafe fn perform_with_sink(operation: &IFileOperation) -> Result<Performed, Error> {
    let sink = ComObject::new(ResultSink { results: RefCell::new(Vec::new()) });
    let sink_interface: IFileOperationProgressSink = sink.to_interface();
    let cookie = operation.Advise(&sink_interface)?;
    let performed = operation.PerformOperations();
    let aborted = operation.GetAnyOperationsAborted().map(|aborted| aborted.as_bool());
    // Detach before any `?`: the operation holds a reference to the sink.
    let unadvised = operation.Unadvise(cookie);
    Ok(Performed { results: last_result_per_item(sink.results.take()), performed, aborted, unadvised })
}

/// What goes into the second, elevated pass.
#[derive(Debug, PartialEq)]
struct RetryPlan {
    /// Indices of the items that go again with an elevation prompt.
    retry: Vec<usize>,
    /// Failures that stand regardless.
    standing: Vec<(PathBuf, HRESULT)>,
}

/// After a first pass: which items go again with an elevation prompt.
///
/// `None` when no item failed for want of rights — then the first pass's
/// result is the answer. Otherwise every item that neither succeeded nor
/// failed for another reason goes again: with `FOFX_EARLYFAILURE` the items
/// after the first failure report nothing, they never ran.
fn plan_elevated_retry(item_keys: &[String], results: &[(PathBuf, HRESULT)]) -> Option<RetryPlan> {
    if !results.iter().any(|(_, code)| needs_elevation(*code)) {
        return None;
    }
    let result_of = |key: &str| results.iter().find(|(path, _)| item_key(path) == key).map(|(_, code)| *code);
    let mut retry = Vec::new();
    let mut standing = Vec::new();
    for (index, key) in item_keys.iter().enumerate() {
        match result_of(key) {
            Some(code) if code.is_ok() => {}
            Some(code) if !needs_elevation(code) => {
                if let Some(failure) = results.iter().find(|(path, _)| item_key(path) == *key) {
                    standing.push(failure.clone());
                }
            }
            _ => retry.push(index),
        }
    }
    // Failures the sink reported under a path none of the items has.
    for failure in results.iter().filter(|(path, code)| code.is_err() && !item_keys.contains(&item_key(path))) {
        if !needs_elevation(failure.1) {
            standing.push(failure.clone());
        }
    }
    Some(RetryPlan { retry, standing })
}

/// The flags of the second, elevated pass: those of the first, elevated as a
/// whole up front, and with the shell allowed to show its error dialogs.
///
/// Measured on Windows 11: `FOFX_SHOWELEVATIONPROMPT` shows the shell's
/// "File Access Denied — You'll need to provide administrator permission"
/// dialog before the UAC prompt, so two questions per operation;
/// `FOFX_REQUIREELEVATION` goes straight to UAC. Both stay silent when
/// `FOF_NOERRORUI` is set (which `FOF_NO_UI` includes): no prompt at all, the
/// item fails as in the first pass.
fn elevated_flags(flags: FILEOPERATION_FLAGS) -> FILEOPERATION_FLAGS {
    FILEOPERATION_FLAGS((flags.0 & !FOF_NOERRORUI.0) | FOFX_REQUIREELEVATION.0)
}

/// Queues `items` on one `IFileOperation` and performs it; when `prompt`
/// allows it, the items that only administrator rights could move go again
/// on a second one that lets the shell ask for them.
///
/// `key` is the path the shell reports an item under, `label` the path an
/// error names it by (a restored item by its original path, not its name in
/// the Recycle Bin), `queue` adds one item to an operation.
unsafe fn perform_items<T>(
    items: &[T],
    key: impl Fn(&T) -> String,
    label: impl Fn(&T) -> PathBuf,
    queue: impl Fn(&IFileOperation, &T) -> Result<(), Error>,
    flags: FILEOPERATION_FLAGS,
    prompt: ElevationPrompt,
) -> Result<(), Error> {
    let run = |items: &mut dyn Iterator<Item = &T>,
               flags: FILEOPERATION_FLAGS,
               owner: Option<HWND>|
     -> Result<Performed, Error> {
        let operation: IFileOperation = CoCreateInstance(&FileOperation as *const _, None, CLSCTX_ALL)?;
        operation.SetOperationFlags(flags)?;
        if let Some(owner) = owner {
            operation.SetOwnerWindow(owner)?;
        }
        for item in items {
            queue(&operation, item)?;
        }
        perform_with_sink(&operation)
    };

    let item_keys: Vec<String> = items.iter().map(&key).collect();
    let relabel = |results: &mut Vec<(PathBuf, HRESULT)>| {
        for (path, _) in results.iter_mut() {
            let reported = item_key(path);
            if let Some(item) = item_keys.iter().position(|key| *key == reported).and_then(|index| items.get(index)) {
                *path = label(item);
            }
        }
    };

    let mut first = run(&mut items.iter(), flags, None)?;
    let ElevationPrompt::OwnedBy(owner) = prompt else {
        relabel(&mut first.results);
        return first.into_result();
    };
    let Some(RetryPlan { retry, mut standing }) = plan_elevated_retry(&item_keys, &first.results) else {
        relabel(&mut first.results);
        return first.into_result();
    };
    relabel(&mut standing);
    if retry.is_empty() {
        return Err(failure_error(&standing));
    }
    let mut second = run(
        &mut retry.iter().filter_map(|&index| items.get(index)),
        elevated_flags(flags),
        Some(HWND(owner as *mut c_void)),
    )?;
    relabel(&mut second.results);
    if standing.is_empty() {
        return second.into_result();
    }
    standing.extend(second.failures());
    Err(failure_error(&standing))
}

fn to_wide_path(path: impl AsRef<OsStr>) -> Vec<u16> {
    path.as_ref().encode_wide().chain(std::iter::once(0)).collect()
}

/// Converts a path to the form accepted by `SHCreateItemFromParsingName`.
///
/// Paths reaching this module have been canonicalized with `Path::canonicalize`.
/// On Windows, this produces verbatim paths, which the shell does not accept.
/// This function converts them back to ordinary Windows paths:
///
/// - Local drive: `\\?\C:\dir\file` becomes `C:\dir\file`.
/// - Network share: `\\?\UNC\host\share\dir\file` becomes `\\host\share\dir\file`.
///
/// Mapped drives resolve to network-share paths during canonicalization.
/// Other path prefixes are left unchanged.
/// The result is encoded as a null-terminated UTF-16 string.
fn to_shell_parsing_name(path: &Path) -> Vec<u16> {
    let mut components = path.components();
    let mut out = OsString::new();
    // A separator goes between two name parts, never right after the prefix
    // (`C:dir` stays drive-relative) or after the root (its backslash is
    // written by the `RootDir` arm).
    let mut needs_separator = false;
    match components.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::VerbatimDisk(letter) => out.push(format!("{}:", letter as char)),
            Prefix::VerbatimUNC(host, share_name) => {
                out.push(r"\\");
                out.push(host);
                out.push(r"\");
                out.push(share_name);
            }
            _ => out.push(prefix.as_os_str()),
        },
        Some(component) => {
            out.push(component.as_os_str());
            needs_separator = true;
        }
        None => {}
    }
    for component in components {
        match component {
            Component::RootDir => {
                out.push(r"\");
                needs_separator = false;
            }
            component => {
                if needs_separator {
                    out.push(r"\");
                }
                out.push(component.as_os_str());
                needs_separator = true;
            }
        }
    }
    to_wide_path(out)
}

#[derive(Clone, Default, Debug)]
pub struct PlatformTrashContext {
    elevation_prompt: ElevationPrompt,
}
impl PlatformTrashContext {
    pub const fn new() -> Self {
        PlatformTrashContext { elevation_prompt: ElevationPrompt::Never }
    }
}
impl TrashContext {
    /// See https://docs.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-_shfileopstructa
    pub(crate) fn delete_specified_canonicalized(&self, full_paths: Vec<PathBuf>) -> Result<(), Error> {
        ensure_com_initialized();
        // The shell's form of each path: it does not take verbatim paths.
        let parsing_names: Vec<Vec<u16>> = full_paths.iter().map(|path| to_shell_parsing_name(path)).collect();
        // https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperation-performoperations
        // PerformOperations can return a success code while items were skipped; the sink reports which and why.
        unsafe {
            let path_of = |name: &Vec<u16>| PathBuf::from(OsString::from_wide(&name[..name.len() - 1]));
            perform_items(
                &parsing_names,
                |name| item_key(&path_of(name)),
                path_of,
                |operation, name| {
                    let item: IShellItem = SHCreateItemFromParsingName(PCWSTR(name.as_ptr()), None)?;
                    operation.DeleteItem(&item, None)?;
                    Ok(())
                },
                FOF_NO_UI | FOF_ALLOWUNDO | FOF_WANTNUKEWARNING,
                self.platform_specific.elevation_prompt,
            )
        }
    }

    /// Removes all files and folder paths recursively.
    pub(crate) fn delete_all_canonicalized(&self, full_paths: Vec<PathBuf>) -> Result<(), Error> {
        self.delete_specified_canonicalized(full_paths)?;
        Ok(())
    }
}

pub fn list() -> Result<Vec<TrashItem>, Error> {
    ensure_com_initialized();
    unsafe {
        let mut item_vec = Vec::new();

        let recycle_bin: IShellItem = SHGetKnownFolderItem(&FOLDERID_RecycleBinFolder, KF_FLAG_DEFAULT, None)?;

        let pesi: IEnumShellItems = recycle_bin.BindToHandler(None, &BHID_EnumItems)?;

        loop {
            let mut fetched_count: u32 = 0;
            let mut arr = [None];
            pesi.Next(&mut arr, Some(&mut fetched_count as *mut u32))?;

            if fetched_count == 0 {
                break;
            }

            match &arr[0] {
                Some(item) => {
                    let id = get_display_name(item, SIGDN_DESKTOPABSOLUTEPARSING)?;
                    let name = get_display_name(item, SIGDN_PARENTRELATIVE)?;
                    let item2: IShellItem2 = item.cast()?;
                    let original_location_variant = item2.GetProperty(&SCID_ORIGINAL_LOCATION)?;
                    let original_location_bstr = PropVariantToBSTR(&original_location_variant)?;
                    let original_location = OsString::from_wide(&original_location_bstr);
                    let date_deleted = get_date_deleted_unix(&item2)?;

                    // NTFS paths are valid Unicode according to this chart:
                    // https://en.wikipedia.org/wiki/Filename#Comparison_of_filename_limitations
                    // Converting a String back to OsString doesn't do extra work
                    item_vec.push(TrashItem {
                        id,
                        name: name.into_string().map_err(|original| Error::ConvertOsString { original })?.into(),
                        original_parent: PathBuf::from(original_location),
                        time_deleted: date_deleted,
                    });
                }
                None => {
                    break;
                }
            }
        }

        Ok(item_vec)
    }
}

pub fn is_empty() -> Result<bool, Error> {
    ensure_com_initialized();
    unsafe {
        let recycle_bin: IShellItem = SHGetKnownFolderItem(&FOLDERID_RecycleBinFolder, KF_FLAG_DEFAULT, None)?;
        let pesi: IEnumShellItems = recycle_bin.BindToHandler(None, &BHID_EnumItems)?;

        let mut count = 0u32;
        let mut items = [None];
        pesi.Next(&mut items, Some(&mut count as *mut u32))?;

        Ok(count == 0)
    }
}

pub fn metadata(item: &TrashItem) -> Result<TrashItemMetadata, Error> {
    ensure_com_initialized();
    let id_as_wide = to_wide_path(&item.id);
    let parsing_name = PCWSTR(id_as_wide.as_ptr());
    let item: IShellItem = unsafe { SHCreateItemFromParsingName(parsing_name, None)? };
    let is_dir = unsafe { item.GetAttributes(SFGAO_FOLDER)? } == SFGAO_FOLDER;
    let size = if is_dir {
        let pesi: IEnumShellItems = unsafe { item.BindToHandler(None, &BHID_EnumItems)? };
        let mut size = 0;
        loop {
            let mut fetched_count: u32 = 0;
            let mut arr = [None];
            unsafe { pesi.Next(&mut arr, Some(&mut fetched_count as *mut u32))? };

            if fetched_count == 0 {
                break;
            }

            match &arr[0] {
                Some(_item) => {
                    size += 1;
                }
                None => {
                    break;
                }
            }
        }
        TrashItemSize::Entries(size)
    } else {
        let item2: IShellItem2 = item.cast()?;
        TrashItemSize::Bytes(unsafe { item2.GetUInt64(&PKEY_Size)? })
    };
    Ok(TrashItemMetadata { size })
}

pub fn purge_all<I>(items: I) -> Result<(), Error>
where
    I: IntoIterator,
    <I as IntoIterator>::Item: Borrow<TrashItem>,
{
    ensure_com_initialized();
    unsafe {
        let pfo: IFileOperation = CoCreateInstance(&FileOperation as *const _, None, CLSCTX_ALL)?;
        pfo.SetOperationFlags(FOF_NO_UI)?;
        let mut at_least_one = false;
        for item in items {
            at_least_one = true;
            let id_as_wide = to_wide_path(&item.borrow().id);
            let parsing_name = PCWSTR(id_as_wide.as_ptr());
            let trash_item: IShellItem = SHCreateItemFromParsingName(parsing_name, None)?;
            pfo.DeleteItem(&trash_item, None)?;
        }
        if at_least_one {
            pfo.PerformOperations()?;
        }
        Ok(())
    }
}

pub fn restore_all<I>(items: I) -> Result<(), Error>
where
    I: IntoIterator<Item = TrashItem>,
{
    restore_all_with(items.into_iter().collect(), ElevationPrompt::Never)
}

fn restore_all_with(items: Vec<TrashItem>, prompt: ElevationPrompt) -> Result<(), Error> {
    // Do a quick and dirty check if the target items already exist at the location
    // and if they do, return all of them, if they don't just go ahead with the processing
    // without giving a damn.
    // Note that this is not 'thread safe' meaning that if a paralell thread (or process)
    // does this operation the exact same time or creates files or folders right after this check,
    // then the files that would collide will not be detected and returned as part of an error.
    // Instead Windows will display a prompt to the user whether they want to replace or skip.
    for item in items.iter() {
        let path = item.original_path();
        if path.exists() {
            return Err(Error::RestoreCollision { path, remaining_items: items });
        }
    }
    if items.is_empty() {
        return Ok(());
    }
    ensure_com_initialized();
    unsafe {
        perform_items(
            &items,
            |item| item_key(Path::new(&item.id)),
            |item| item.original_path(),
            |operation, item| {
                let id_as_wide = to_wide_path(&item.id);
                let parsing_name = PCWSTR(id_as_wide.as_ptr());
                let trash_item: IShellItem = SHCreateItemFromParsingName(parsing_name, None)?;
                let parent_path_wide = to_wide_path(&item.original_parent);
                let orig_folder_shi: IShellItem = SHCreateItemFromParsingName(PCWSTR(parent_path_wide.as_ptr()), None)?;
                let name_wstr = to_wide_path(&item.name);

                operation.MoveItem(&trash_item, &orig_folder_shi, PCWSTR(name_wstr.as_ptr()), None)?;
                Ok(())
            },
            FOF_NO_UI | FOFX_EARLYFAILURE,
            prompt,
        )
    }
}

unsafe fn get_display_name(psi: &IShellItem, sigdnname: SIGDN) -> Result<OsString, Error> {
    let name = psi.GetDisplayName(sigdnname)?;
    let result = wstr_to_os_string(name);
    CoTaskMemFree(Some(name.0 as *const c_void));
    Ok(result)
}

unsafe fn wstr_to_os_string(wstr: PWSTR) -> OsString {
    let mut len = 0;
    while *(wstr.0.offset(len)) != 0 {
        len += 1;
    }
    let wstr_slice = std::slice::from_raw_parts(wstr.0, len as usize);
    OsString::from_wide(wstr_slice)
}

unsafe fn get_date_deleted_unix(item: &IShellItem2) -> Result<i64, Error> {
    /// January 1, 1970 as Windows file time
    const EPOCH_AS_FILETIME: u64 = 116444736000000000;
    const HUNDREDS_OF_NANOSECONDS: u64 = 10000000;

    let time = item.GetFileTime(&SCID_DATE_DELETED)?;
    let time_u64 = ((time.dwHighDateTime as u64) << 32) | (time.dwLowDateTime as u64);
    let rel_to_linux_epoch = time_u64 - EPOCH_AS_FILETIME;
    let seconds_since_unix_epoch = rel_to_linux_epoch / HUNDREDS_OF_NANOSECONDS;

    Ok(seconds_since_unix_epoch as i64)
}

struct CoInitializer {}
impl CoInitializer {
    fn new() -> CoInitializer {
        //let first = INITIALIZER_THREAD_COUNT.fetch_add(1, Ordering::SeqCst) == 0;
        #[cfg(all(not(feature = "coinit_multithreaded"), not(feature = "coinit_apartmentthreaded")))]
        {
            0 = "THIS IS AN ERROR ON PURPOSE. Either the `coinit_multithreaded` or the `coinit_apartmentthreaded` feature must be specified";
        }
        let mut init_mode;
        #[cfg(feature = "coinit_multithreaded")]
        {
            init_mode = COINIT_MULTITHREADED;
        }
        #[cfg(feature = "coinit_apartmentthreaded")]
        {
            init_mode = COINIT_APARTMENTTHREADED;
        }

        // These flags can be combined with either of coinit_multithreaded or coinit_apartmentthreaded.
        if cfg!(feature = "coinit_disable_ole1dde") {
            init_mode |= COINIT_DISABLE_OLE1DDE;
        }
        if cfg!(feature = "coinit_speed_over_memory") {
            init_mode |= COINIT_SPEED_OVER_MEMORY;
        }
        let hr = unsafe { CoInitializeEx(None, init_mode) };
        if hr.is_err() {
            panic!("Call to CoInitializeEx failed. HRESULT: {:?}. Consider using `trash` with the feature `coinit_multithreaded`", hr);
        }
        CoInitializer {}
    }
}
impl Drop for CoInitializer {
    fn drop(&mut self) {
        // TODO: This does not get called because it's a global static.
        // Is there an atexit in Win32?
        unsafe {
            CoUninitialize();
        }
    }
}
thread_local! {
    static CO_INITIALIZER: CoInitializer = CoInitializer::new();
}
fn ensure_com_initialized() {
    CO_INITIALIZER.with(|_| {});
}

#[cfg(test)]
mod tests {
    use super::{copy_engine_reason, to_shell_parsing_name};
    use std::path::Path;
    use windows::core::HRESULT;

    use super::{item_key, last_result_per_item, plan_elevated_retry, RetryPlan};
    use std::path::PathBuf;

    const ACCESS_DENIED_SRC: HRESULT = HRESULT(0x8027_0021_u32 as i32);
    const ACCESS_DENIED_DEST: HRESULT = HRESULT(0x8027_0022_u32 as i32);
    const REQUIRES_ELEVATION: HRESULT = HRESULT(0x8027_0002_u32 as i32);
    const SHARING_VIOLATION_SRC: HRESULT = HRESULT(0x8027_0027_u32 as i32);
    const DONT_PROCESS_CHILDREN: HRESULT = HRESULT(0x0027_0008);

    fn keys(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|path| item_key(Path::new(path))).collect()
    }

    fn result(path: &str, code: HRESULT) -> (PathBuf, HRESULT) {
        (PathBuf::from(path), code)
    }

    #[test]
    fn the_elevated_pass_keeps_the_flags_but_lets_uac_through() {
        use super::elevated_flags;
        use windows::Win32::UI::Shell::*;
        // Restore: what went straight to one UAC prompt and restored the item
        // in the measurement.
        assert_eq!(elevated_flags(FOF_NO_UI | FOFX_EARLYFAILURE).0, 0x1010_0214);
        // Delete: FOF_NO_UI without FOF_NOERRORUI, elevated up front.
        assert_eq!(
            elevated_flags(FOF_NO_UI | FOF_ALLOWUNDO | FOF_WANTNUKEWARNING),
            FOF_SILENT
                | FOF_NOCONFIRMATION
                | FOF_NOCONFIRMMKDIR
                | FOF_ALLOWUNDO
                | FOF_WANTNUKEWARNING
                | FOFX_REQUIREELEVATION
        );
    }

    #[test]
    fn an_item_counts_with_its_last_result() {
        // What the shell reported for one item with an elevation prompt
        // (measured): twice REQUIRES_ELEVATION, then recycled.
        let results = last_result_per_item(vec![
            result(r"C:\Users\x.txt", REQUIRES_ELEVATION),
            result(r"C:\Users\x.txt", REQUIRES_ELEVATION),
            result(r"c:\users\X.TXT", DONT_PROCESS_CHILDREN),
            result(r"C:\Users\y.txt", ACCESS_DENIED_SRC),
        ]);
        assert_eq!(
            results,
            vec![result(r"C:\Users\x.txt", DONT_PROCESS_CHILDREN), result(r"C:\Users\y.txt", ACCESS_DENIED_SRC)]
        );
    }

    #[test]
    fn no_second_pass_without_a_lack_of_rights() {
        let items = keys(&[r"C:\a.txt", r"C:\b.txt"]);
        let results = [result(r"C:\a.txt", DONT_PROCESS_CHILDREN), result(r"C:\b.txt", SHARING_VIOLATION_SRC)];
        assert_eq!(plan_elevated_retry(&items, &results), None);
    }

    #[test]
    fn a_refused_item_goes_again() {
        // The first pass of the measurement: one admin-owned file.
        let items = keys(&[r"\\?\C:\Users\Stefan"]);
        let results = [result(r"C:\Users\Stefan", ACCESS_DENIED_SRC)];
        assert_eq!(plan_elevated_retry(&items, &results), Some(RetryPlan { retry: vec![0], standing: vec![] }));
    }

    #[test]
    fn items_after_an_early_failure_go_again_too() {
        // Restoring with FOFX_EARLYFAILURE: the second item never ran.
        let items = keys(&[r"C:\$Recycle.Bin\S-1\$RA.txt", r"C:\$Recycle.Bin\S-1\$RB.txt"]);
        let results = [result(r"C:\$Recycle.Bin\S-1\$RA.txt", ACCESS_DENIED_DEST)];
        assert_eq!(plan_elevated_retry(&items, &results), Some(RetryPlan { retry: vec![0, 1], standing: vec![] }));
    }

    #[test]
    fn other_failures_stand_and_successes_stay_done() {
        let items = keys(&[r"C:\done.txt", r"C:\admin.txt", r"C:\in-use.txt"]);
        let results = [
            result(r"C:\done.txt", DONT_PROCESS_CHILDREN),
            result(r"C:\admin.txt", ACCESS_DENIED_SRC),
            result(r"C:\in-use.txt", SHARING_VIOLATION_SRC),
        ];
        assert_eq!(
            plan_elevated_retry(&items, &results),
            Some(RetryPlan { retry: vec![1], standing: vec![result(r"C:\in-use.txt", SHARING_VIOLATION_SRC)] })
        );
    }

    #[test]
    fn copy_engine_failures_get_their_own_words() {
        let reason = |code: u32| copy_engine_reason(HRESULT(code as i32));
        assert_eq!(reason(0x8027_0002), Some("the operation requires elevation"));
        assert_eq!(reason(0x8027_0021), Some("access denied to the source"));
        // Unlisted copy engine failures still are not handed to FormatMessage.
        assert_eq!(reason(0x8027_00FF), Some("the shell's file operation failed"));
        // A success code of the copy engine is no failure.
        assert_eq!(reason(0x0027_0008), None);
        // Other facilities: the system's own message is right.
        assert_eq!(reason(0x8007_0005), None);
    }

    fn convert(path: &str) -> String {
        let wide = to_shell_parsing_name(Path::new(path));
        String::from_utf16(&wide[..wide.len() - 1]).unwrap()
    }

    #[test]
    fn verbatim_disk_becomes_plain_disk() {
        assert_eq!(convert(r"\\?\C:\dir\file.txt"), r"C:\dir\file.txt");
    }

    #[test]
    fn verbatim_unc_becomes_plain_unc() {
        // What `canonicalize` returns for a mapped drive or a UNC path.
        // Before the fix this came out as `UNC\host\share\dir\file.txt`,
        // which the shell reads as a relative path (issue #55).
        assert_eq!(convert(r"\\?\UNC\host\share\dir\file.txt"), r"\\host\share\dir\file.txt");
    }

    #[test]
    fn non_verbatim_paths_are_unchanged() {
        assert_eq!(convert(r"C:\dir\file.txt"), r"C:\dir\file.txt");
        assert_eq!(convert(r"\\host\share\dir\file.txt"), r"\\host\share\dir\file.txt");
        assert_eq!(convert(r"dir\file.txt"), r"dir\file.txt");
    }

    #[test]
    fn empty_path_stays_empty() {
        assert_eq!(convert(""), "");
    }

    #[test]
    fn drive_root_keeps_its_backslash() {
        // `C:` alone would mean the current directory on C:, not the root.
        assert_eq!(convert(r"\\?\C:\"), r"C:\");
        assert_eq!(convert(r"C:\"), r"C:\");
    }

    #[test]
    fn share_root_with_and_without_trailing_backslash() {
        assert_eq!(convert(r"\\?\UNC\host\share"), r"\\host\share");
        assert_eq!(convert(r"\\?\UNC\host\share\"), r"\\host\share\");
    }

    #[test]
    fn drive_relative_path_gets_no_separator_after_the_colon() {
        assert_eq!(convert(r"C:dir\file.txt"), r"C:dir\file.txt");
    }

    #[test]
    fn other_verbatim_and_device_prefixes_are_unchanged() {
        assert_eq!(convert(r"\\?\pictures\file.txt"), r"\\?\pictures\file.txt");
        assert_eq!(convert(r"\\.\pipe\name"), r"\\.\pipe\name");
    }

    #[test]
    fn spaces_dots_and_non_ascii_names_survive() {
        assert_eq!(
            convert(r"\\?\UNC\nas-01\My Photos\2026.09 trip\日本語 ファイル.jpg"),
            r"\\nas-01\My Photos\2026.09 trip\日本語 ファイル.jpg"
        );
        assert_eq!(convert(r"\\?\C:\a.b\c..d\file.tar.gz"), r"C:\a.b\c..d\file.tar.gz");
    }

    #[test]
    fn trailing_and_doubled_separators_are_dropped() {
        // `components()` normalizes these; the shell accepts either form.
        assert_eq!(convert(r"\\?\C:\dir\"), r"C:\dir");
        assert_eq!(convert(r"\\?\C:\dir\\file.txt"), r"C:\dir\file.txt");
        assert_eq!(convert(r"\\?\UNC\host\share\dir\\file.txt"), r"\\host\share\dir\file.txt");
    }

    #[test]
    fn long_paths_are_not_truncated() {
        let deep = (0..40).map(|i| format!("folder{i:02}")).collect::<Vec<_>>().join(r"\");
        let input = format!(r"\\?\UNC\host\share\{deep}\file.txt");
        let expected = format!(r"\\host\share\{deep}\file.txt");
        assert!(input.len() > 260);
        assert_eq!(convert(&input), expected);
    }
}
