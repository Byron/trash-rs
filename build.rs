#[cfg(not(target_os = "windows"))]
fn main() {}

#[cfg(target_os = "windows")]
fn main() {
    windows::build!(
        Windows::Win32::SystemServices::PWSTR,
        Windows::Win32::WindowsAndMessaging::HWND,
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
            IFileOperationProgressSink
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
