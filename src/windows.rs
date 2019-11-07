use std::ffi::{OsStr, OsString};
use std::ops::DerefMut;
use std::os::windows::prelude::*;
use std::path::{Path, PathBuf};
use std::mem::MaybeUninit;

use scopeguard::defer;

use winapi::DEFINE_GUID;
use winapi::{
    ctypes::{c_int, c_void},
    shared::guiddef::REFIID,
    shared::minwindef::{DWORD, FILETIME, LPVOID, UINT},
    shared::windef::HWND,
    shared::winerror::{HRESULT_FROM_WIN32, SUCCEEDED, S_OK},
    shared::wtypes::{VT_BSTR, VT_DATE},
    um::combaseapi::{CoInitializeEx, CoTaskMemFree, CoUninitialize, CoCreateInstance, CLSCTX_ALL},
    um::errhandlingapi::GetLastError,
    um::minwinbase::SYSTEMTIME,
    um::oaidl::VARIANT,
    um::objbase::{
        COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, COINIT_MULTITHREADED,
        COINIT_SPEED_OVER_MEMORY,
    },
    um::oleauto::{VariantChangeType, VariantClear, VariantTimeToSystemTime},
    um::shellapi::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_NO_UI, FOF_SILENT, FOF_WANTNUKEWARNING, FO_DELETE,
        SHFILEOPSTRUCTW,
    },
    um::winuser::{CreatePopupMenu, DestroyMenu},
    um::shlobj::CSIDL_BITBUCKET,
    um::shlwapi::StrRetToStrW,
    um::shobjidl_core::{
        IContextMenu, IEnumIDList, IShellFolder, IShellFolder2, IFileOperation, FileOperation, IShellItem, SHCONTF_FOLDERS, SHCONTF_NONFOLDERS, SHGDNF,
        SHGDN_FORPARSING, SHGDN_INFOLDER, CMF_NORMAL, CMINVOKECOMMANDINFO, CMINVOKECOMMANDINFOEX, CMIC_MASK_FLAG_NO_UI, SHCreateItemWithParent
    },
    um::shtypes::{PCUITEMID_CHILD, PCUITEMID_CHILD_ARRAY, PIDLIST_RELATIVE, PIDLIST_ABSOLUTE, PITEMID_CHILD, SHCOLUMNID, STRRET},
    um::timezoneapi::SystemTimeToFileTime,
    um::winnt::{PCZZWSTR, PWSTR, ULARGE_INTEGER},
    Interface,
    Class
};

use crate::{Error, TrashItem};

macro_rules! return_err_on_fail {
    {$f_name:ident($($args:tt)*)} => ({
        let hr = $f_name($($args)*);
        if !SUCCEEDED(hr) {
            return Err(Error::PlatformApi {
                function_name: stringify!($f_name).into(),
                code: Some(hr)
            });
        }
        hr
    });
    {$obj:ident.$f_name:ident($($args:tt)*)} => ({
        return_err_on_fail!{($obj).$f_name($($args)*)}
    });
    {($obj:expr).$f_name:ident($($args:tt)*)} => ({
        let hr = ($obj).$f_name($($args)*);
        if !SUCCEEDED(hr) {
            return Err(Error::PlatformApi {
                function_name: stringify!($f_name).into(),
                code: Some(hr)
            });
        }
        hr
    })
}

pub fn remove_all<I, T>(paths: I) -> Result<(), Error>
where
    I: IntoIterator<Item = T>,
    T: AsRef<Path>,
{
    let paths = paths.into_iter();
    let full_paths = paths
        .map(|x| x.as_ref().canonicalize())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| Error::CanonicalizePath {
            code: e.raw_os_error(),
        })?;
    let mut wide_paths = Vec::with_capacity(full_paths.len());
    for path in full_paths.iter() {
        let mut os_string = OsString::from(path);
        os_string.push("\0");
        let mut encode_wide = os_string.as_os_str().encode_wide();
        // Remove the "\\?\" prefix as `SHFileOperationW` fails if such a prefix is part of the path.
        // See:
        // https://docs.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-_shfileopstructa
        assert_eq!(encode_wide.next(), Some('\\' as u16));
        assert_eq!(encode_wide.next(), Some('\\' as u16));
        assert_eq!(encode_wide.next(), Some('?' as u16));
        assert_eq!(encode_wide.next(), Some('\\' as u16));
        let mut wide_path: Vec<_> = encode_wide.collect();
        wide_paths.append(&mut wide_path);
    }
    wide_paths.push(0); // The string has to be double zero terminated.

    let mut fileop = SHFILEOPSTRUCTW {
        hwnd: 0 as HWND,
        wFunc: FO_DELETE as UINT,
        pFrom: wide_paths.as_ptr() as PCZZWSTR,
        pTo: std::ptr::null(),
        fFlags: FOF_ALLOWUNDO | FOF_SILENT | FOF_WANTNUKEWARNING,
        fAnyOperationsAborted: 0,
        hNameMappings: std::ptr::null_mut(),
        lpszProgressTitle: std::ptr::null(),
    };

    let result = unsafe { SHFileOperationW(&mut fileop as *mut SHFILEOPSTRUCTW) };

    if result == S_OK {
        Ok(())
    } else {
        Err(Error::Remove { code: Some(result) })
    }
}

/// See https://docs.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-_shfileopstructa
pub fn remove<T: AsRef<Path>>(path: T) -> Result<(), Error> {
    remove_all(&[path])
}

pub fn list() -> Result<Vec<TrashItem>, Error> {
    ensure_com_initialized();
    unsafe {
        let mut recycle_bin: *mut IShellFolder2 = std::mem::uninitialized();
        bind_to_csidl(
            CSIDL_BITBUCKET,
            &IShellFolder2::uuidof() as *const _,
            &mut recycle_bin as *mut *mut _ as *mut *mut c_void,
        )?;
        defer! {{ (*recycle_bin).Release(); }};
        let mut peidl: *mut IEnumIDList = std::mem::uninitialized();
        let hr = return_err_on_fail! {
            (*recycle_bin).EnumObjects(
                std::ptr::null_mut(),
                SHCONTF_FOLDERS | SHCONTF_NONFOLDERS,
                &mut peidl as *mut _,
            )
        };
        if hr != S_OK {
            return Err(Error::PlatformApi {
                function_name: "EnumObjects".into(),
                code: Some(hr),
            });
        }
        let mut item_vec = Vec::new();
        let mut item: PITEMID_CHILD = std::mem::uninitialized();
        while (*peidl).Next(1, &mut item as *mut _, std::ptr::null_mut()) == S_OK {
            defer! {{ CoTaskMemFree(item as LPVOID); }}
            let id = get_display_name(recycle_bin as *mut _, item, SHGDN_FORPARSING)?;
            let name = get_display_name(recycle_bin as *mut _, item, SHGDN_INFOLDER)?;

            let orig_loc = get_detail(recycle_bin, item, &SCID_ORIGINAL_LOCATION as *const _)?;
            let date_deleted = get_date_unix(recycle_bin, item, &SCID_DATE_DELETED as *const _)?;

            item_vec.push(TrashItem {
                id,
                name: name
                    .into_string()
                    .map_err(|original| Error::ConvertOsString { original })?,
                original_parent: PathBuf::from(orig_loc),
                time_deleted: date_deleted,
            });
        }
        return Ok(item_vec);
    }
}

pub fn purge_all<I>(items: I) -> Result<(), Error>
where
    I: IntoIterator<Item = TrashItem>,
{
    ensure_com_initialized();
    unsafe {
        let mut recycle_bin = MaybeUninit::<*mut IShellFolder2>::uninit();
        bind_to_csidl(
            CSIDL_BITBUCKET,
            &IShellFolder2::uuidof() as *const _,
            recycle_bin.as_mut_ptr() as *mut *mut c_void,
        )?;
        let recycle_bin = recycle_bin.assume_init();
        defer! {{ (*recycle_bin).Release(); }}
        let mut pfo = MaybeUninit::<*mut IFileOperation>::uninit();
        return_err_on_fail! {
            CoCreateInstance(
                &FileOperation::uuidof() as *const _,
                std::ptr::null_mut(),
                CLSCTX_ALL,
                &IFileOperation::uuidof() as *const _,
                pfo.as_mut_ptr() as *mut *mut c_void,
            )
        };
        let pfo = pfo.assume_init();
        defer!{{ (*pfo).Release(); }}
        return_err_on_fail! { (*pfo).SetOperationFlags(FOF_NO_UI as DWORD) };
        for item in items {
            let mut id_wstr: Vec<_> = item.id.encode_wide().chain(std::iter::once(0)).collect();
            let mut pidl = MaybeUninit::<PIDLIST_RELATIVE>::uninit();
            return_err_on_fail! {
                (*recycle_bin).ParseDisplayName(
                    0 as _,
                    std::ptr::null_mut(), 
                    id_wstr.as_mut_ptr(),
                    std::ptr::null_mut(),
                    pidl.as_mut_ptr(),
                    std::ptr::null_mut(),
                )
            };
            let pidl = pidl.assume_init();
            defer! {{ CoTaskMemFree(pidl as LPVOID); }}
            let mut shi = MaybeUninit::<*mut IShellItem>::uninit();
            return_err_on_fail! {
                SHCreateItemWithParent(
                    std::ptr::null_mut(),
                    recycle_bin as *mut _,
                    pidl,
                    &IShellItem::uuidof() as *const _,
                    shi.as_mut_ptr() as *mut *mut c_void,
                )
            };
            let shi = shi.assume_init();
            defer!{{ (*shi).Release(); }}
            return_err_on_fail! { (*pfo).DeleteItem(shi, std::ptr::null_mut()) };
        }
        return_err_on_fail! { (*pfo).PerformOperations() };
        Ok(())
    }
}

pub fn restore_all<I>(items: I) -> Result<(), Error>
where
    I: IntoIterator<Item = TrashItem>,
{
    execute_verb_on_all(items.into_iter(), "undelete")
}

struct CoInitializer {}
impl CoInitializer {
    fn new() -> CoInitializer {
        //let first = INITIALIZER_THREAD_COUNT.fetch_add(1, Ordering::SeqCst) == 0;
        let mut init_mode = 0;
        if cfg!(coinit_multithreaded) {
            init_mode |= COINIT_MULTITHREADED;
        } else if cfg!(coinit_apartmentthreaded) {
            init_mode |= COINIT_APARTMENTTHREADED;
        }
        // else missing intentionaly. These flags can be combined
        if cfg!(coinit_disable_ole1dde) {
            init_mode |= COINIT_DISABLE_OLE1DDE;
        }
        if cfg!(coinit_speed_over_memory) {
            init_mode |= COINIT_SPEED_OVER_MEMORY;
        }
        let hr = unsafe { CoInitializeEx(std::ptr::null_mut(), init_mode) };
        if !SUCCEEDED(hr) {
            panic!(format!("Call to CoInitializeEx failed. HRESULT: {:X}. Consider using `trash` with the feature `coinit_multithreaded`", hr));
        }
        CoInitializer {}
    }
}
impl Drop for CoInitializer {
    fn drop(&mut self) {
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

unsafe fn bind_to_csidl(csidl: c_int, riid: REFIID, ppv: *mut *mut c_void) -> Result<(), Error> {
    use winapi::um::shlobj::{SHGetDesktopFolder, SHGetSpecialFolderLocation};

    let mut pidl: PIDLIST_ABSOLUTE = std::mem::uninitialized();
    return_err_on_fail! {
        SHGetSpecialFolderLocation(std::ptr::null_mut(), csidl, &mut pidl as *mut _)
    };
    defer! {{ CoTaskMemFree(pidl as LPVOID); }};
    let mut desktop: *mut IShellFolder = std::mem::uninitialized();
    return_err_on_fail! {SHGetDesktopFolder(&mut desktop as *mut *mut _)};
    defer! {{ (*desktop).Release(); }};
    if (*pidl).mkid.cb != 0 {
        return_err_on_fail! {(*desktop).BindToObject(pidl, std::ptr::null_mut(), riid, ppv)};
    } else {
        return_err_on_fail! {(*desktop).QueryInterface(riid, ppv)};
    }
    Ok(())
}

unsafe fn wstr_to_os_string(wstr: PWSTR) -> OsString {
    let mut len = 0;
    while *(wstr.offset(len)) != 0 {
        len += 1;
    }
    let wstr_slice = std::slice::from_raw_parts(wstr, len as usize);
    OsString::from_wide(wstr_slice)
}

unsafe fn get_display_name(
    psf: *mut IShellFolder,
    pidl: PCUITEMID_CHILD,
    flags: SHGDNF,
) -> Result<OsString, Error> {
    let mut sr: STRRET = std::mem::uninitialized();
    return_err_on_fail! {(*psf).GetDisplayNameOf(pidl, flags, &mut sr as *mut _)};
    let mut name: PWSTR = std::mem::uninitialized();
    return_err_on_fail! {StrRetToStrW(&mut sr as *mut _, pidl, &mut name as *mut _)};
    let result = wstr_to_os_string(name);
    CoTaskMemFree(name as LPVOID);
    Ok(result)
}

unsafe fn get_detail(
    psf: *mut IShellFolder2,
    pidl: PCUITEMID_CHILD,
    pscid: *const SHCOLUMNID,
) -> Result<OsString, Error> {
    let mut vt: VARIANT = std::mem::uninitialized();
    return_err_on_fail! { (*psf).GetDetailsEx(pidl, pscid, &mut vt as *mut _) };
    let mut vt = scopeguard::guard(vt, |mut vt| {
        VariantClear(&mut vt as *mut _);
    });
    //defer! {{ VariantClear(&mut vt as *mut _); }};
    return_err_on_fail! {
        VariantChangeType(vt.deref_mut() as *mut _, vt.deref_mut() as *mut _, 0, VT_BSTR as u16)
    };
    let a = vt.n1.n2().n3.bstrVal();
    let result = Ok(wstr_to_os_string(*a));
    return result;
}

fn windows_ticks_to_unix_seconds(windows_ticks: u64) -> i64 {
    const WINDOWS_TICK: u64 = 10000000;
    const SEC_TO_UNIX_EPOCH: i64 = 11644473600;
    return (windows_ticks / WINDOWS_TICK) as i64 - SEC_TO_UNIX_EPOCH;
}

unsafe fn variant_time_to_unix_time(from: f64) -> Result<i64, Error> {
    let mut st: SYSTEMTIME = std::mem::MaybeUninit::uninit().assume_init();
    return_err_on_fail! { VariantTimeToSystemTime(from, &mut st as *mut _) };
    let mut ft: FILETIME = std::mem::MaybeUninit::uninit().assume_init();
    if SystemTimeToFileTime(&st, &mut ft as *mut _) == 0 {
        Err(Error::PlatformApi {
            function_name: "SystemTimeToFileTime".into(),
            code: Some(HRESULT_FROM_WIN32(GetLastError())),
        })
    } else {
        let mut uli = std::mem::MaybeUninit::<ULARGE_INTEGER>::zeroed().assume_init();
        {
            let u_mut = uli.u_mut();
            u_mut.LowPart = ft.dwLowDateTime;
            u_mut.HighPart = std::mem::transmute(ft.dwHighDateTime);
        }
        let windows_ticks: u64 = *uli.QuadPart();
        Ok(windows_ticks_to_unix_seconds(windows_ticks))
    }
}

unsafe fn get_date_unix(
    psf: *mut IShellFolder2,
    pidl: PCUITEMID_CHILD,
    pscid: *const SHCOLUMNID,
) -> Result<i64, Error> {
    let mut vt: VARIANT = std::mem::uninitialized();
    return_err_on_fail! { (*psf).GetDetailsEx(pidl, pscid, &mut vt as *mut _) };
    let mut vt = scopeguard::guard(vt, |mut vt| {
        VariantClear(&mut vt as *mut _);
    });
    return_err_on_fail! {
        VariantChangeType(vt.deref_mut() as *mut _, vt.deref_mut() as *mut _, 0, VT_DATE as u16)
    };
    let date = *vt.n1.n2().n3.date();
    let unix_time = variant_time_to_unix_time(date)?;
    Ok(unix_time)
}

unsafe fn invoke_verb(pcm: *mut IContextMenu, verb: &'static str) -> Result<(), Error> {
	// Recycle bin verbs:
	// undelete
	// cut
	// delete
	// properties
	let hmenu = CreatePopupMenu();
	if hmenu == std::ptr::null_mut() {
        return Err(Error::PlatformApi {
            function_name: "CreatePopupMenu".into(),
            code: Some(HRESULT_FROM_WIN32(GetLastError())),
        });
    }
    defer! {{ DestroyMenu(hmenu); }}
    return_err_on_fail!{(*pcm).QueryContextMenu(hmenu, 0, 1, 0x7FFF, CMF_NORMAL)};
    let zero_terminated_verb: Vec<_> = verb.bytes().chain(std::iter::once(0)).collect();
    let mut info = MaybeUninit::<CMINVOKECOMMANDINFOEX>::zeroed().assume_init();
    info.cbSize = std::mem::size_of::<CMINVOKECOMMANDINFOEX>() as u32;
    info.lpVerb = zero_terminated_verb.as_ptr() as *const i8;
    info.fMask = CMIC_MASK_FLAG_NO_UI;
    return_err_on_fail!{(*pcm).InvokeCommand(&mut info as *mut _ as *mut _)};
    Ok(())
}

fn execute_verb_on_all(
    items: impl Iterator<Item = TrashItem>,
    verb: &'static str,
) -> Result<(), Error> {
    ensure_com_initialized();
    unsafe {
        let mut recycle_bin = MaybeUninit::<*mut IShellFolder2>::uninit();
        bind_to_csidl(
            CSIDL_BITBUCKET,
            &IShellFolder2::uuidof() as *const _,
            recycle_bin.as_mut_ptr() as *mut *mut c_void,
        )?;
        let recycle_bin = recycle_bin.assume_init();
        defer! {{ (*recycle_bin).Release(); }};
        let mut items_pidl = scopeguard::guard(Vec::new(), |items_pidl| {
            for &pidl in items_pidl.iter() {
                CoTaskMemFree(pidl as LPVOID);
            }
        });
        for item in items {
            let mut id_wstr: Vec<_> = item.id.encode_wide().chain(std::iter::once(0)).collect();
            let mut pidl = MaybeUninit::<PIDLIST_RELATIVE>::uninit();
            return_err_on_fail! {
                (*recycle_bin).ParseDisplayName(
                    0 as _,
                    std::ptr::null_mut(), 
                    id_wstr.as_mut_ptr(),
                    std::ptr::null_mut(),
                    pidl.as_mut_ptr(),
                    std::ptr::null_mut(),
                )
            };
            items_pidl.push(pidl.assume_init());
        }
        if items_pidl.len() > 0 {
            let mut pcm = MaybeUninit::<*mut IContextMenu>::uninit();
            //IContextMenu* pcm;
            return_err_on_fail! {
                (*recycle_bin).GetUIObjectOf(
                    0 as _,
                    items_pidl.len() as u32,
                    items_pidl.as_ptr() as PCUITEMID_CHILD_ARRAY,
                    &IID_IContextMenu as *const _,
                    std::ptr::null_mut(),
                    pcm.as_mut_ptr() as *mut _
                )
            };
            let pcm = pcm.assume_init();
            defer! {{ (*pcm).Release(); }}
            invoke_verb(pcm, verb)?;
        }
        Ok(())
    }
}

DEFINE_GUID! {
    PSGUID_DISPLACED,
    0x9b174b33, 0x40ff, 0x11d2, 0xa2, 0x7e, 0x00, 0xc0, 0x4f, 0xc3, 0x8, 0x71
}

// TODO MOVE THIS TO WINAPI-RS
DEFINE_GUID! {
    IID_IContextMenu,
    0x000214e4, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46
}

const PID_DISPLACED_FROM: DWORD = 2;
const PID_DISPLACED_DATE: DWORD = 3;

const SCID_ORIGINAL_LOCATION: SHCOLUMNID = SHCOLUMNID {
    fmtid: PSGUID_DISPLACED,
    pid: PID_DISPLACED_FROM,
};
const SCID_DATE_DELETED: SHCOLUMNID = SHCOLUMNID {
    fmtid: PSGUID_DISPLACED,
    pid: PID_DISPLACED_DATE,
};
