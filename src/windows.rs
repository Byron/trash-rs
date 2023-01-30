use crate::{Error, TrashContext, TrashItem};
use std::{
    ffi::{c_void, OsStr, OsString},
    os::windows::{ffi::OsStrExt, prelude::*},
    path::PathBuf,
};
use windows::core::{Interface, GUID, PCWSTR, PWSTR};
use windows::Win32::{Foundation::*, System::Com::*, UI::Shell::PropertiesSystem::*, UI::Shell::*};

///////////////////////////////////////////////////////////////////////////
// These don't have bindings in windows-rs for some reason
///////////////////////////////////////////////////////////////////////////
const PSGUID_DISPLACED: GUID =
    GUID::from_values(0x9b174b33, 0x40ff, 0x11d2, [0xa2, 0x7e, 0x00, 0xc0, 0x4f, 0xc3, 0x8, 0x71]);
const PID_DISPLACED_FROM: u32 = 2;
const PID_DISPLACED_DATE: u32 = 3;
const SCID_ORIGINAL_LOCATION: PROPERTYKEY =
    PROPERTYKEY { fmtid: PSGUID_DISPLACED, pid: PID_DISPLACED_FROM };
const SCID_DATE_DELETED: PROPERTYKEY =
    PROPERTYKEY { fmtid: PSGUID_DISPLACED, pid: PID_DISPLACED_DATE };

const FOF_SILENT: u32 = 0x0004;
const FOF_NOCONFIRMATION: u32 = 0x0010;
const FOF_ALLOWUNDO: u32 = 0x0040;
const FOF_NOCONFIRMMKDIR: u32 = 0x0200;
const FOF_NOERRORUI: u32 = 0x0400;
const FOF_WANTNUKEWARNING: u32 = 0x4000;
const FOF_NO_UI: u32 = FOF_SILENT | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_NOCONFIRMMKDIR;
const FOFX_EARLYFAILURE: u32 = 0x00100000;
///////////////////////////////////////////////////////////////////////////

impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Error {
        Error::Unknown { description: format!("windows error: {err}") }
    }
}

#[derive(Clone, Default, Debug)]
pub struct PlatformTrashContext;
impl PlatformTrashContext {
    pub const fn new() -> Self {
        PlatformTrashContext
    }
}
impl TrashContext {
    /// See https://docs.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-_shfileopstructa
    pub(crate) fn delete_all_canonicalized(&self, full_paths: Vec<PathBuf>) -> Result<(), Error> {
        ensure_com_initialized();
        unsafe {
            let pfo: IFileOperation =
                CoCreateInstance(&FileOperation as *const _, None, CLSCTX_ALL).unwrap();

            pfo.SetOperationFlags(FOF_NO_UI | FOF_ALLOWUNDO | FOF_WANTNUKEWARNING)?;

            for full_path in full_paths.iter() {
                let path_prefix = ['\\' as u16, '\\' as u16, '?' as u16, '\\' as u16];
                let wide_path_container: Vec<_> =
                    full_path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
                let wide_path_slice = if wide_path_container.starts_with(&path_prefix) {
                    &wide_path_container[path_prefix.len()..]
                } else {
                    &wide_path_container[0..]
                };

                let shi: IShellItem =
                    SHCreateItemFromParsingName(PCWSTR(wide_path_slice.as_ptr()), None)?;

                pfo.DeleteItem(&shi, None)?;
            }
            pfo.PerformOperations()?;
            Ok(())
        }
    }
}

pub fn list() -> Result<Vec<TrashItem>, Error> {
    ensure_com_initialized();
    unsafe {
        let mut item_vec = Vec::new();

        let recycle_bin: IShellItem =
            SHGetKnownFolderItem(&FOLDERID_RecycleBinFolder, KF_FLAG_DEFAULT, HANDLE::default())?;

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
                    let original_location = OsString::from_wide(original_location_bstr.as_wide());
                    let date_deleted = get_date_deleted_unix(&item2)?;

                    item_vec.push(TrashItem {
                        id,
                        name: name
                            .into_string()
                            .map_err(|original| Error::ConvertOsString { original })?,
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

pub fn purge_all<I>(items: I) -> Result<(), Error>
where
    I: IntoIterator<Item = TrashItem>,
{
    ensure_com_initialized();
    unsafe {
        let pfo: IFileOperation = CoCreateInstance(&FileOperation as *const _, None, CLSCTX_ALL)?;
        pfo.SetOperationFlags(FOF_NO_UI)?;
        let mut at_least_one = false;
        for item in items {
            at_least_one = true;
            let id_as_wide: Vec<u16> = item.id.encode_wide().chain(std::iter::once(0)).collect();
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
    let items: Vec<_> = items.into_iter().collect();

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
    ensure_com_initialized();
    unsafe {
        let pfo: IFileOperation = CoCreateInstance(&FileOperation as *const _, None, CLSCTX_ALL)?;
        pfo.SetOperationFlags(FOF_NO_UI | FOFX_EARLYFAILURE)?;
        for item in items.iter() {
            let id_as_wide: Vec<u16> = item.id.encode_wide().chain(std::iter::once(0)).collect();
            let parsing_name = PCWSTR(id_as_wide.as_ptr());
            let trash_item: IShellItem = SHCreateItemFromParsingName(parsing_name, None)?;
            let parent_path_wide: Vec<_> =
                item.original_parent.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
            let orig_folder_shi: IShellItem =
                SHCreateItemFromParsingName(PCWSTR(parent_path_wide.as_ptr()), None)?;
            let name_wstr: Vec<_> = AsRef::<OsStr>::as_ref(&item.name)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            pfo.MoveItem(&trash_item, &orig_folder_shi, PCWSTR(name_wstr.as_ptr()), None)?;
        }
        if !items.is_empty() {
            pfo.PerformOperations()?;
        }
        Ok(())
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
        #[cfg(all(
            not(feature = "coinit_multithreaded"),
            not(feature = "coinit_apartmentthreaded")
        ))]
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
