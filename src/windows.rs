use std::{
    ffi::{c_void, OsStr, OsString},
    fmt::format,
    mem::MaybeUninit,
    ops::DerefMut,
    os::{
        raw::c_int,
        windows::{ffi::OsStrExt, prelude::*},
    },
    path::{Path, PathBuf},
    ptr::null_mut,
};

use scopeguard::defer;

mod bindings {
    ::windows::include_bindings!();
}

use bindings::Windows::Win32::{Com::*, Shell::*, SystemServices::*, WindowsAndMessaging::*};
use windows::{Abi, IUnknown, Interface, IntoParam, Param, RuntimeType};

struct WinNull;
impl<'a> IntoParam<'a, IBindCtx> for WinNull {
    fn into_param(self) -> Param<'a, IBindCtx> {
        Param::None
    }
}
impl<'a> IntoParam<'a, IUnknown> for WinNull {
    fn into_param(self) -> Param<'a, IUnknown> {
        Param::None
    }
}
impl<'a> IntoParam<'a, IFileOperationProgressSink> for WinNull {
    fn into_param(self) -> Param<'a, IFileOperationProgressSink> {
        Param::None
    }
}

// These don't have bindings in windows-rs for some reason:
const FOF_SILENT: u32 = 0x0004;
const FOF_RENAMEONCOLLISION: u32 = 0x0008;
const FOF_NOCONFIRMATION: u32 = 0x0010;
const FOF_WANTMAPPINGHANDLE: u32 = 0x0020;
const FOF_ALLOWUNDO: u32 = 0x0040;
const FOF_FILESONLY: u32 = 0x0080;
const FOF_SIMPLEPROGRESS: u32 = 0x0100;
const FOF_NOCONFIRMMKDIR: u32 = 0x0200;
const FOF_NOERRORUI: u32 = 0x0400;
const FOF_NOCOPYSECURITYATTRIBS: u32 = 0x0800;
const FOF_NORECURSION: u32 = 0x1000;
const FOF_NO_CONNECTED_ELEMENTS: u32 = 0x2000;
const FOF_WANTNUKEWARNING: u32 = 0x4000;
const FOF_NO_UI: u32 = FOF_SILENT | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_NOCONFIRMMKDIR;

use crate::{Error, TrashItem};

macro_rules! return_err_on_fail {
    {$f_name:ident($($args:tt)*)} => ({
        let hr = $f_name($($args)*);
        if hr.is_err() {
            return Err(Error::Unknown {
                description: format!("`{}` failed with the result: {:?}", stringify!($f_name), hr)
            });
        }
        // hr
    });
    {$obj:ident.$f_name:ident($($args:tt)*)} => ({
        return_err_on_fail!{($obj).$f_name($($args)*)}
    });
    {($obj:expr).$f_name:ident($($args:tt)*)} => ({
        let hr = ($obj).$f_name($($args)*);
        if hr.is_err() {
            return Err(Error::Unknown {
                description: format!("`{}` failed with the result: {:?}", stringify!($f_name), hr)
            });
        }
        // hr
    })
}

/// See https://docs.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-_shfileopstructa
pub fn delete_all_canonicalized(full_paths: Vec<PathBuf>) -> Result<(), Error> {
    ensure_com_initialized();
    unsafe {
        let recycle_bin: IShellFolder2 = bind_to_csidl(CSIDL_BITBUCKET as c_int)?;
        // let mut pbc = MaybeUninit::<*mut IBindCtx>::uninit();
        // return_err_on_fail! { CreateBindCtx(0, pbc.as_mut_ptr()) };
        // let pbc = pbc.assume_init();
        // defer! {{ (*pbc).Release(); }}
        // (*pbc).
        let mut pfo = MaybeUninit::<IFileOperation>::uninit();
        return_err_on_fail! {
            CoCreateInstance(
                &FileOperation as *const _,
                WinNull,
                CLSCTX::CLSCTX_ALL,
                &IFileOperation::IID as *const _,
                pfo.as_mut_ptr() as *mut *mut c_void,
            )
        };
        let pfo = pfo.assume_init();
        return_err_on_fail! { pfo.SetOperationFlags(FOF_NO_UI | FOF_ALLOWUNDO | FOF_WANTNUKEWARNING) };
        for full_path in full_paths.iter() {
            let path_prefix = ['\\' as u16, '\\' as u16, '?' as u16, '\\' as u16];
            let mut wide_path_container: Vec<_> =
                full_path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
            let wide_path_slice = if wide_path_container.starts_with(&path_prefix) {
                &mut wide_path_container[path_prefix.len()..]
            } else {
                &mut wide_path_container[0..]
            };
            let mut shi = MaybeUninit::<IShellItem>::uninit();
            return_err_on_fail! {
                SHCreateItemFromParsingName(
                    PWSTR(wide_path_slice.as_mut_ptr()),
                    WinNull,
                    &IShellItem::IID as *const _,
                    shi.as_mut_ptr() as *mut *mut c_void,
                )
            };
            let shi = shi.assume_init();
            return_err_on_fail! { pfo.DeleteItem(shi, WinNull) };
        }
        return_err_on_fail! { pfo.PerformOperations() };
        Ok(())
    }
}

unsafe fn bind_to_csidl<T: Interface>(csidl: c_int) -> Result<T, Error> {
    let mut pidl = MaybeUninit::<*mut ITEMIDLIST>::uninit();
    return_err_on_fail! {
        SHGetSpecialFolderLocation(HWND::NULL, csidl, pidl.as_mut_ptr())
    };
    let pidl = pidl.assume_init();
    defer! {{ CoTaskMemFree(pidl as _); }};

    let mut desktop = MaybeUninit::<Option<IShellFolder>>::uninit();
    return_err_on_fail! { SHGetDesktopFolder(desktop.as_mut_ptr()) };
    let desktop = desktop.assume_init();
    let desktop = desktop.ok_or_else(|| Error::Unknown {
        description: "`SHGetDesktopFolder` set its output to `None`.".into(),
    })?;
    if (*pidl).mkid.cb != 0 {
        let iid = T::IID;
        // let bind_ctx = MaybeUninit::<Option<IBindCtx>>::uninit();
        // return_err_on_fail! { CreateBindCtx(0, bind_ctx.as_mut_ptr()) };
        // let bind_ctx = bind_ctx.assume_init().ok_or_else(|| Error::Unknown {
        //     description: "CreateBindCtx returned None".into()
        // })?;

        // WARNING The following logic relies on the fact that T has an identical memory
        // layout to a pointer, and is treated like a pointer by the `windows-rs` implementation.
        // This logic follows how the IUnknown::cast function is implemented in windows-rs 0.8
        let mut target = MaybeUninit::<T>::uninit();
        return_err_on_fail! { desktop.BindToObject(pidl, WinNull, &iid as *const _, target.as_mut_ptr() as *mut *mut c_void) };
        Ok(target.assume_init())
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
            init_mode = COINIT::COINIT_MULTITHREADED;
        }
        #[cfg(feature = "coinit_apartmentthreaded")]
        {
            init_mode = COINIT::COINIT_APARTMENTTHREADED;
        }

        // These flags can be combined with either of coinit_multithreaded or coinit_apartmentthreaded.
        if cfg!(feature = "coinit_disable_ole1dde") {
            init_mode |= COINIT::COINIT_DISABLE_OLE1DDE;
        }
        if cfg!(feature = "coinit_speed_over_memory") {
            init_mode |= COINIT::COINIT_SPEED_OVER_MEMORY;
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

fn into_unknown<E: std::fmt::Display>(err: E) -> Error {
    Error::Unknown { description: format!("{}", err) }
}
