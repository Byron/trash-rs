#[cfg(not(target_os = "windows"))]
fn main() {}

#[cfg(target_os = "windows")]
fn main() {
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
