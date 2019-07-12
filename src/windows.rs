use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use winapi::{
    shared::minwindef::UINT,
    shared::windef::HWND,
    shared::winerror::S_OK,
    um::shellapi::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_SILENT, FOF_WANTNUKEWARNING, FO_DELETE,
        SHFILEOPSTRUCTW,
    },
    um::winnt::PCZZWSTR,
};

use crate::Error;

pub fn is_implemented() -> bool {
    true
}

/// See https://docs.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-_shfileopstructa
pub fn remove<T: AsRef<Path>>(path: T) -> Result<(), Error> {
    let canonical = path
        .as_ref()
        .canonicalize()
        .map_err(|e| Error::CanonicalizePath {
            code: e.raw_os_error(),
        })?;
    let mut from = OsString::from(canonical);
    from.push("\0\0"); // The string has to be double zero terminated.

    let mut encode_wide = from.as_os_str().encode_wide();

    // Remove the "\\?\" prefix as `SHFileOperationW` fails if such a prefix is part of the path.
    // See:
    // https://docs.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-_shfileopstructa
    assert_eq!(encode_wide.next(), Some('\\' as u16));
    assert_eq!(encode_wide.next(), Some('\\' as u16));
    assert_eq!(encode_wide.next(), Some('?' as u16));
    assert_eq!(encode_wide.next(), Some('\\' as u16));
    let wide_path: Vec<_> = encode_wide.collect();

    let mut fileop = SHFILEOPSTRUCTW {
        hwnd: 0 as HWND,
        wFunc: FO_DELETE as UINT,
        pFrom: wide_path.as_ptr() as PCZZWSTR,
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
