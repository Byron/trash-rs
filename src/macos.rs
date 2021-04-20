use std::{ffi::OsString, path::PathBuf, process::Command};

use log::{trace, warn};
use objc::{
    class, msg_send,
    runtime::{Object, BOOL, NO},
    sel, sel_impl,
};

use crate::{into_unknown, Error, TrashContext};

#[link(name = "Foundation", kind = "framework")]
extern "C" {
    // Using an empty scope to just link against the foundation framework,
    // to find the NSFileManager, but we don't need anything else from it.
}

#[allow(non_camel_case_types)]
type id = *mut Object;
#[allow(non_upper_case_globals)]
const nil: id = std::ptr::null_mut();
#[allow(non_upper_case_globals)]
const NSUTF8StringEncoding: usize = 4;

#[derive(Copy, Clone, Debug)]
pub enum DeleteMethod {
    /// Use an `osascript`, asking the Finder application to delete the files.
    ///
    /// - Might ask the user to give additional permissions to the app
    /// - Produces the sound that Finder usually makes when deleting a file
    /// - Shows the "Put Back" option in the context menu, when using the Finder application
    ///
    /// This is the default.
    Finder,

    /// Use `trashItemAtURL` from the `NSFileManager` object to delete the files.
    ///
    /// - Somewhat faster than the `Finder` method
    /// - Does *not* require additional permissions
    /// - Does *not* produce the sound that Finder usually makes when deleting a file
    /// - Does *not* show the "Put Back" option on some systems (the file may be restored by for
    ///   example dragging out from the Trash folder). This is a macOS bug. Read more about it
    ///   at:
    ///   - <https://github.com/sindresorhus/macos-trash/issues/4>
    ///   - <https://github.com/ArturKovacs/trash-rs/issues/14>
    NsFileManager,
}
impl DeleteMethod {
    /// Returns `DeleteMethod::Finder`
    pub const fn new() -> Self {
        DeleteMethod::Finder
    }
}
impl Default for DeleteMethod {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Clone, Default, Debug)]
pub struct PlatformTrashContext {
    delete_method: DeleteMethod,
}
impl PlatformTrashContext {
    pub const fn new() -> Self {
        Self { delete_method: DeleteMethod::new() }
    }
}
pub trait TrashContextExtMacos {
    fn set_delete_method(&mut self, method: DeleteMethod);
    fn delete_method(&self) -> DeleteMethod;
}
impl TrashContextExtMacos for TrashContext {
    fn set_delete_method(&mut self, method: DeleteMethod) {
        self.platform_specific.delete_method = method;
    }
    fn delete_method(&self) -> DeleteMethod {
        self.platform_specific.delete_method
    }
}
impl TrashContext {
    pub(crate) fn delete_all_canonicalized(&self, full_paths: Vec<PathBuf>) -> Result<(), Error> {
        let full_paths = full_paths.into_iter().map(to_string).collect::<Result<Vec<_>, _>>()?;
        match self.platform_specific.delete_method {
            DeleteMethod::Finder => delete_using_finder(full_paths),
            DeleteMethod::NsFileManager => delete_using_file_mgr(full_paths),
        }
    }
}

fn delete_using_file_mgr(full_paths: Vec<String>) -> Result<(), Error> {
    trace!("Starting delete_using_file_mgr");
    let url_cls = class!(NSURL);
    let file_mgr_cls = class!(NSFileManager);
    let file_mgr: id = unsafe { msg_send![file_mgr_cls, defaultManager] };
    for path in full_paths {
        let string = to_ns_string(&path);
        trace!("Starting fileURLWithPath");
        let url: id = unsafe { msg_send![url_cls, fileURLWithPath:string.ptr] };
        if url == nil {
            return Err(Error::Unknown {
                description: format!("Failed to convert a path to an NSURL. Path: '{}'", path),
            });
        }
        trace!("Finished fileURLWithPath");
        // WARNING: I don't know why but if we try to call release on the url, it sometimes
        // crashes with SIGSEGV, so we instead don't try to release the url
        // let url = OwnedObject { ptr: url };
        let mut error: id = nil;
        trace!("Calling trashItemAtURL");
        let success: BOOL = unsafe {
            msg_send![
                file_mgr,
                trashItemAtURL:url
                resultingItemURL:nil
                error:(&mut error as *mut id)
            ]
        };
        trace!("Finished trashItemAtURL");
        if success == NO {
            trace!("success was NO");
            if error == nil {
                return Err(Error::Unknown {
                    description: format!(
                        "While deleting '{}', `trashItemAtURL` returned with failure but no error was specified.",
                        path
                    )
                });
            }
            let code: isize = unsafe { msg_send![error, code] };
            let domain: id = unsafe { msg_send![error, domain] };
            let domain = unsafe { ns_string_to_rust(domain)? };
            return Err(Error::Unknown {
                description: format!(
                    "While deleting '{}', `trashItemAtURL` failed, code: {}, domain: {}",
                    path, code, domain
                ),
            });
        }
    }
    Ok(())
}

fn delete_using_finder(full_paths: Vec<String>) -> Result<(), Error> {
    // AppleScript command to move files (or directories) to Trash looks like
    //   osascript -e 'tell application "Finder" to delete { POSIX file "file1", POSIX "file2" }'
    // The `-e` flag is used to execute only one line of AppleScript.
    let mut command = Command::new("osascript");
    let posix_files = full_paths
        .into_iter()
        .map(|p| format!("POSIX file \"{}\"", p))
        .collect::<Vec<String>>()
        .join(", ");
    let script = format!("tell application \"Finder\" to delete {{ {} }}", posix_files);

    let argv: Vec<OsString> = vec!["-e".into(), script.into()];
    command.args(argv);

    // Execute command
    let result = command.output().map_err(into_unknown)?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(Error::Unknown {
            description: format!(
                "The AppleScript exited unsuccesfully. Error code: {:?}, stderr: {}",
                result.status.code(),
                stderr
            ),
        });
    }
    Ok(())
}

fn to_string<T: Into<OsString>>(str_in: T) -> Result<String, Error> {
    let os_string = str_in.into();
    let s = os_string.to_str();
    match s {
        Some(s) => Ok(s.to_owned()),
        None => Err(Error::ConvertOsString { original: os_string }),
    }
}

/// Uses the Drop trait to `release` the object held by `ptr`.
#[repr(transparent)]
struct OwnedObject {
    pub ptr: id,
}
impl Drop for OwnedObject {
    fn drop(&mut self) {
        let () = unsafe { msg_send![self.ptr, release] };
    }
}

fn to_ns_string(s: &str) -> OwnedObject {
    trace!("Called to_ns_string on '{}'", s);
    let utf8 = s.as_bytes();
    let string_cls = class!(NSString);
    let alloced_string: id = unsafe { msg_send![string_cls, alloc] };
    let mut string: id = unsafe {
        msg_send![
            alloced_string,
            initWithBytes:utf8.as_ptr()
            length:utf8.len()
            encoding:NSUTF8StringEncoding
        ]
    };
    if string == nil {
        warn!("initWithBytes returned nil when trying to convert a rust string to an NSString");
        string = unsafe { msg_send![alloced_string, init] };
    }
    OwnedObject { ptr: string }
}

/// Safety: `string` is assumed to be a pointer to an NSString
unsafe fn ns_string_to_rust(string: id) -> Result<String, Error> {
    if string == nil {
        return Ok(String::new());
    }
    let utf8_bytes: *const u8 = msg_send![string, UTF8String];
    let utf8_len: usize = msg_send![string, lengthOfBytesUsingEncoding: NSUTF8StringEncoding];
    let str_slice = std::slice::from_raw_parts(utf8_bytes, utf8_len);
    let rust_str = std::str::from_utf8(str_slice).map_err(into_unknown)?;
    Ok(rust_str.to_owned())
}
