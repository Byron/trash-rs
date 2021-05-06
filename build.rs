use std::ffi::OsStr;

fn generate_windows_bindings() {
    windows::build!(
        Windows::Win32::SystemServices::{PWSTR, S_OK},
        Windows::Win32::WindowsProgramming::{
            SYSTEMTIME,
            FILETIME,
            SystemTimeToFileTime
        },
        Windows::Win32::WindowsAndMessaging::HWND,
        Windows::Win32::WindowsPropertiesSystem::PROPERTYKEY,
        Windows::Win32::Automation::{
            VARIANT,
            VariantClear,
            VariantChangeType,
            VARENUM,
            VariantTimeToSystemTime
        },
        Windows::Win32::Shell::{
            SHGetDesktopFolder,
            IShellFolder2,
            SHGetSpecialFolderLocation,
            SHCreateItemFromParsingName,
            CSIDL_BITBUCKET,
            ITEMIDLIST,
            IFileOperation,
            FileOperation,
            IShellItem,
            IFileOperationProgressSink,
            IEnumIDList,
            _SHCONTF,
            STRRET,
            StrRetToStrW,
            _SHGDNF,
            SHCreateItemWithParent
        },
        Windows::Win32::Com::{
            CoInitializeEx,
            CoUninitialize,
            COINIT,
            CoTaskMemFree,
            CoTaskMemFree,
            CoCreateInstance,
            CLSCTX,
            CreateBindCtx,
            IBindCtx
        },
    );
}

fn main() {
    let targeting_windows = {
        if let Some(target) = std::env::var_os("CARGO_CFG_TARGET_OS") {
            target.as_os_str() == OsStr::new("windows")
        } else {
            false
        }
    };
    if targeting_windows {
        generate_windows_bindings();
    }
}
