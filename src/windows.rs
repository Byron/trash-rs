use std::{
    ffi::{c_void, OsStr, OsString},
    mem::{size_of, MaybeUninit},
    ops::DerefMut,
    os::{
        raw::c_int,
        windows::{ffi::OsStrExt, prelude::*},
    },
    path::PathBuf,
};

use scopeguard::defer;
use windows::core::{Interface, GUID, PCWSTR, PWSTR};

use crate::{into_unknown, Error, TrashContext, TrashItem};

use windows::Win32::{
    Foundation::*, System::Com::*, System::Ole::*, System::Time::*, UI::Shell::Common::*,
    UI::Shell::PropertiesSystem::*, UI::Shell::*,
};

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
// {43826D1E-E718-42EE-BC55-A1E261C37BFE}
const IID_IShellItem: GUID =
    GUID::from_values(0x43826d1e, 0xe718, 0x42ee, [0xbc, 0x55, 0xa1, 0xe2, 0x61, 0xc3, 0x7b, 0xfe]);

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
        Error::Unknown { description: format!("windows error: {}", err.to_string()) }
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
                let mut wide_path_container: Vec<_> =
                    full_path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
                let wide_path_slice = if wide_path_container.starts_with(&path_prefix) {
                    &wide_path_container[path_prefix.len()..]
                } else {
                    &wide_path_container[0..]
                };

                let shi: IShellItem =
                    SHCreateItemFromParsingName(PCWSTR(wide_path_slice.as_ptr()), None)?;

                pfo.DeleteItem(shi, None)?;
            }
            pfo.PerformOperations()?;
            Ok(())
        }
    }
}

pub fn list() -> Result<Vec<TrashItem>, Error> {
    ensure_com_initialized();
    unsafe {
        // let recycle_bin: IShellFolder2 = bind_to_csidl(CSIDL_BITBUCKET as c_int)?;
        // let mut peidl = MaybeUninit::<Option<IEnumIDList>>::uninit();
        // let flags = SHCONTF_FOLDERS.0 | SHCONTF_NONFOLDERS.0;

        // let hr = recycle_bin.EnumObjects(HWND::default(), flags as u32, peidl.as_mut_ptr());
        // // WARNING `hr.is_ok()` is DIFFERENT from `hr == S_OK`, because
        // // `is_ok` returns true if the HRESULT as any of the several success codes
        // // but here we want to be more strict and only accept S_OK.
        // if hr != S_OK {
        //     return Err(Error::Unknown {
        //         description: format!(
        //             "`EnumObjects` returned with HRESULT {:X}, but 0x0 was expected.",
        //             hr.0
        //         ),
        //     });
        // }
        // let peidl = peidl.assume_init().ok_or_else(|| Error::Unknown {
        //     description: "`EnumObjects` set its output to None.".into(),
        // })?;
        let mut item_vec = Vec::new();

        // TODO: make sure we free this
        let mut recycle_bin = MaybeUninit::<Option<IShellItem2>>::uninit();

        SHGetKnownFolderItem(
            &FOLDERID_RecycleBinFolder,
            KF_FLAG_DEFAULT,
            HANDLE::default(),
            &IID_IShellItem,
            recycle_bin.as_mut_ptr() as *mut *mut c_void,
        )?;

        let recycle_bin = recycle_bin.assume_init().expect("not initialized");

        let pesi: IEnumShellItems = recycle_bin.BindToHandler(None, &BHID_EnumItems)?;
        let mut fetched: u32 = 0;

        loop {
            let mut arr = [None];
            pesi.Next(&mut arr, &mut fetched)?;

            if fetched == 0 {
                break;
            }

            // arr.len()
            match &arr[0] {
                Some(item) => {
                    // TODO: is SIGDN_DESKTOPABSOLUTEPARSING equivalent to SHGDN_FORPARSING?
                    let id = get_display_name_new(item, SIGDN_DESKTOPABSOLUTEPARSING)?;
                    dbg!(&id);
                    let name = get_display_name_new(item, SIGDN_PARENTRELATIVE)?;
                    dbg!(&name);
                    let item2: IShellItem2 = item.cast()?;
                    // TODO: free this?
                    let original_location_variant = item2.GetProperty(&SCID_ORIGINAL_LOCATION)?;
                    let original_location_bstr = PropVariantToBSTR(&original_location_variant)?;
                    let original_location = OsString::from_wide(original_location_bstr.as_wide());

                    // let date_deleted_variant = item2.GetProperty(&SCID_DATE_DELETED)?;
                    // let date_deleted = PropVariantToFileTime(&date_deleted_variant, PSTIME_FLAGS)?;
                    let date_deleted = get_date_unix_new(&item2)?;
                    // dbg!(date_deleted);

                    item_vec.push(TrashItem {
                        id,
                        name: name
                            .into_string()
                            .map_err(|original| Error::ConvertOsString { original })?,
                        original_parent: PathBuf::from(original_location),
                        time_deleted: date_deleted, // TODO get actual value
                    });
                    // PropVariantChangeType()
                    // dbg!(original_location);
                }
                None => {
                    break;
                }
            }
        }

        // while let Ok(_) = peidl.Next(&mut [item_uninit.as_mut_ptr()], &mut count) {
        //     if count > 0 {
        //         let mut item = item_uninit.assume_init();

        //         let id = get_display_name((&recycle_bin).into(), &mut item, SHGDN_FORPARSING)?;
        //         let name = get_display_name((&recycle_bin).into(), &mut item, SHGDN_INFOLDER)?;

        //         let orig_loc =
        //             get_detail(&recycle_bin, &mut item, &SCID_ORIGINAL_LOCATION as *const _)?;
        //         let date_deleted =
        //             get_date_unix(&recycle_bin, &mut item, &SCID_DATE_DELETED as *const _)?;

        //         item_vec.push(TrashItem {
        //             id,
        //             name: name
        //                 .into_string()
        //                 .map_err(|original| Error::ConvertOsString { original })?,
        //             original_parent: PathBuf::from(orig_loc),
        //             time_deleted: date_deleted,
        //         });
        //     }
        // }
        Ok(item_vec)
    }
}

pub fn purge_all<I>(items: I) -> Result<(), Error>
where
    I: IntoIterator<Item = TrashItem>,
{
    ensure_com_initialized();
    unsafe {
        let recycle_bin: IShellFolder2 = bind_to_csidl(CSIDL_BITBUCKET as i32)?;
        let pfo: IFileOperation = CoCreateInstance(&FileOperation as *const _, None, CLSCTX_ALL)?;
        pfo.SetOperationFlags(FOF_NO_UI)?;
        let mut at_least_one = false;
        for item in items {
            at_least_one = true;
            let mut id_wstr: Vec<_> = item.id.encode_wide().chain(std::iter::once(0)).collect();
            let mut pidl = MaybeUninit::<*mut ITEMIDLIST>::uninit();
            recycle_bin.ParseDisplayName(
                HWND::default(),
                None,
                PCWSTR(id_wstr.as_mut_ptr()),
                std::ptr::null_mut(),
                pidl.as_mut_ptr(),
                std::ptr::null_mut(),
            )?;
            let pidl = pidl.assume_init();
            defer! {{ CoTaskMemFree(pidl as *mut c_void); }}

            let trash_item = CoTaskMemAlloc(size_of::<IShellItem>()) as *mut IShellItem;
            defer! {{ CoTaskMemFree(trash_item as *mut c_void); }}

            SHCreateItemWithParent(
                std::ptr::null(),
                &recycle_bin,
                pidl,
                &IShellItem::IID,
                &mut (trash_item as *mut c_void),
            )?;

            pfo.DeleteItem(&*trash_item, None)?;
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
        let recycle_bin: IShellFolder2 = bind_to_csidl(CSIDL_BITBUCKET as i32)?;
        let pfo: IFileOperation = CoCreateInstance(&FileOperation as *const _, None, CLSCTX_ALL)?;
        pfo.SetOperationFlags(FOF_NO_UI | FOFX_EARLYFAILURE)?;
        for item in items.iter() {
            let id_wstr: Vec<_> = item.id.encode_wide().chain(std::iter::once(0)).collect();
            let mut pidl = MaybeUninit::<*mut ITEMIDLIST>::uninit();
            recycle_bin.ParseDisplayName(
                HWND::default(),
                None,
                PCWSTR(id_wstr.as_ptr()),
                std::ptr::null_mut(),
                pidl.as_mut_ptr(),
                std::ptr::null_mut(),
            )?;
            let pidl = pidl.assume_init();
            defer! {{ CoTaskMemFree(pidl as *mut c_void); }}

            let trash_item = CoTaskMemAlloc(size_of::<IShellItem>()) as *mut IShellItem;
            defer! {{ CoTaskMemFree(trash_item as *mut c_void); }}

            SHCreateItemWithParent(
                std::ptr::null(),
                &recycle_bin,
                pidl,
                &IShellItem::IID,
                &mut (trash_item as *mut c_void),
            )?;

            let parent_path_wide: Vec<_> =
                item.original_parent.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
            let orig_folder_shi: IShellItem =
                SHCreateItemFromParsingName(PCWSTR(parent_path_wide.as_ptr()), None)?;
            let name_wstr: Vec<_> = AsRef::<OsStr>::as_ref(&item.name)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            pfo.MoveItem(&*trash_item, orig_folder_shi, PCWSTR(name_wstr.as_ptr()), None)?;
        }
        if !items.is_empty() {
            pfo.PerformOperations()?;
        }
        Ok(())
    }
}

unsafe fn get_display_name(
    psf: IShellFolder,
    pidl: *mut ITEMIDLIST,
    flags: _SHGDNF,
) -> Result<OsString, Error> {
    let mut sr = psf.GetDisplayNameOf(pidl, flags.0 as u32)?;
    let mut name = MaybeUninit::<PWSTR>::uninit();
    StrRetToStrW(&mut sr as *mut _, pidl, name.as_mut_ptr())?;
    let name = name.assume_init();
    let result = wstr_to_os_string(name);
    CoTaskMemFree(name.0 as *mut c_void);
    Ok(result)
}

unsafe fn get_display_name_new(psi: &IShellItem, sigdnname: SIGDN) -> Result<OsString, Error> {
    let name = psi.GetDisplayName(sigdnname)?;
    let ret = wstr_to_os_string(name);
    // todo: is this safe? not totally sure whether wstr_to_os_string is cloning the string
    CoTaskMemFree(name.0 as *const c_void);
    Ok(ret)
}

unsafe fn wstr_to_os_string(wstr: PWSTR) -> OsString {
    let mut len = 0;
    while *(wstr.0.offset(len)) != 0 {
        len += 1;
    }
    let wstr_slice = std::slice::from_raw_parts(wstr.0, len as usize);
    OsString::from_wide(wstr_slice)
}

unsafe fn get_detail(
    psf: &IShellFolder2,
    pidl: *mut ITEMIDLIST,
    pscid: *const PROPERTYKEY,
) -> Result<OsString, Error> {
    let vt = psf.GetDetailsEx(pidl, pscid)?;
    let mut vt = scopeguard::guard(vt, |mut vt| {
        // Ignoring the return value
        let _ = VariantClear(&mut vt as *mut _);
    });
    VariantChangeType(vt.deref_mut() as *mut _, vt.deref_mut() as *mut _, 0, VT_BSTR.0 as u16)?;
    let pstr = &vt.Anonymous.Anonymous.Anonymous.bstrVal;
    Ok(OsString::from_wide(pstr.as_wide()))
}

unsafe fn get_date_unix_new(
    item: &IShellItem2
) -> Result<i64, Error> {
    // let vt = psf.GetDetailsEx(pidl, pscid)?;
    let vt = item.GetProperty(&SCID_DATE_DELETED)?;
    
    // let mut vt = scopeguard::guard(vt, |mut vt| {
    //     // Ignoring the return value
    //     let _ = VariantClear(&mut vt as *mut _);
    // });
    // VariantChangeType(vt.deref_mut() as *mut _, vt.deref_mut() as *mut _, 0, VT_DATE.0 as u16)?;
    let date = vt.Anonymous.Anonymous.Anonymous.date;
    let unix_time = variant_time_to_unix_time(date)?;
    Ok(unix_time)
}

unsafe fn get_date_unix(
    psf: &IShellFolder2,
    pidl: *mut ITEMIDLIST,
    pscid: *const PROPERTYKEY,
) -> Result<i64, Error> {
    let vt = psf.GetDetailsEx(pidl, pscid)?;
    let mut vt = scopeguard::guard(vt, |mut vt| {
        // Ignoring the return value
        let _ = VariantClear(&mut vt as *mut _);
    });
    VariantChangeType(vt.deref_mut() as *mut _, vt.deref_mut() as *mut _, 0, VT_DATE.0 as u16)?;
    let date = vt.Anonymous.Anonymous.Anonymous.date;
    let unix_time = variant_time_to_unix_time(date)?;
    Ok(unix_time)
}

unsafe fn variant_time_to_unix_time(from: f64) -> Result<i64, Error> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct LargeIntegerParts {
        low_part: u32,
        high_part: u32,
    }
    #[repr(C)]
    union LargeInteger {
        parts: LargeIntegerParts,
        whole: u64,
    }
    let mut st = MaybeUninit::<SYSTEMTIME>::uninit();
    if 0 == VariantTimeToSystemTime(from, st.as_mut_ptr()) {
        return Err(Error::Unknown {
            description: format!(
                "`VariantTimeToSystemTime` indicated failure for the parameter {:?}",
                from
            ),
        });
    }
    let st = st.assume_init();
    let mut ft = MaybeUninit::<FILETIME>::uninit();
    if SystemTimeToFileTime(&st, ft.as_mut_ptr()) == false {
        return Err(Error::Unknown {
            description: format!("`SystemTimeToFileTime` failed with: {:?}", GetLastError()),
        });
    }
    let ft = ft.assume_init();

    let large_int = LargeInteger {
        parts: LargeIntegerParts { low_part: ft.dwLowDateTime, high_part: ft.dwHighDateTime },
    };

    // Applying assume init straight away because there's no explicit support to initialize struct
    // fields one-by-one in an `MaybeUninit` as of Rust 1.39.0
    // See: https://github.com/rust-lang/rust/blob/1.39.0/src/libcore/mem/maybe_uninit.rs#L170
    // let mut uli = MaybeUninit::<ULARGE_INTEGER>::zeroed().assume_init();
    // {
    //     let u_mut = uli.u_mut();
    //     u_mut.LowPart = ft.dwLowDateTime;
    //     u_mut.HighPart = std::mem::transmute(ft.dwHighDateTime);
    // }
    let windows_ticks: u64 = large_int.whole;
    Ok(windows_ticks_to_unix_seconds(windows_ticks))
}

fn windows_ticks_to_unix_seconds(windows_ticks: u64) -> i64 {
    // Fun fact: if my calculations are correct, then storing sucn ticks in an
    // i64 can remain valid until about 6000 years from the very first tick
    const WINDOWS_TICK: u64 = 10000000;
    const SEC_TO_UNIX_EPOCH: i64 = 11644473600;
    (windows_ticks / WINDOWS_TICK) as i64 - SEC_TO_UNIX_EPOCH
}

unsafe fn bind_to_csidl<T: Interface>(csidl: c_int) -> Result<T, Error> {
    let pidl = SHGetSpecialFolderLocation(HWND::default(), csidl)?;
    defer! {{ CoTaskMemFree(pidl as _); }};

    let desktop = SHGetDesktopFolder()?;
    if (*pidl).mkid.cb != 0 {
        let target: T = desktop.BindToObject(pidl, None)?;
        Ok(target)
    } else {
        Ok(desktop.cast().map_err(into_unknown)?)
    }
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
        let hr = unsafe { CoInitializeEx(std::ptr::null_mut(), init_mode) };
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
