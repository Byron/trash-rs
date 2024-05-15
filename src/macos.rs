use std::{ffi::OsString, path::PathBuf, process::Command};

use log::trace;
use objc2_foundation::{NSFileManager, NSString, NSURL};

use crate::{into_unknown, Error, TrashContext};

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
    let file_mgr = unsafe { NSFileManager::defaultManager() };
    for path in full_paths {
        let string = NSString::from_str(&path);

        trace!("Starting fileURLWithPath");
        let url = unsafe { NSURL::fileURLWithPath(&string) };
        trace!("Finished fileURLWithPath");

        trace!("Calling trashItemAtURL");
        let res = unsafe { file_mgr.trashItemAtURL_resultingItemURL_error(&url, None) };
        trace!("Finished trashItemAtURL");

        if let Err(err) = res {
            return Err(Error::Unknown {
                description: format!("While deleting '{path}', `trashItemAtURL` failed: {err}"),
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
    let posix_files = full_paths.into_iter().map(|p| format!("POSIX file \"{p}\"")).collect::<Vec<String>>().join(", ");
    let script = format!("tell application \"Finder\" to delete {{ {posix_files} }}");

    let argv: Vec<OsString> = vec!["-e".into(), script.into()];
    command.args(argv);

    // Execute command
    let result = command.output().map_err(into_unknown)?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        match result.status.code() {
            None => {
                return Err(Error::Unknown {
                    description: format!("The AppleScript exited with error. stderr: {}", stderr),
                })
            }

            Some(code) => {
                return Err(Error::Os {
                    code,
                    description: format!("The AppleScript exited with error. stderr: {}", stderr),
                })
            }
        };
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

#[cfg(test)]
mod tests {
    use crate::{
        macos::{DeleteMethod, TrashContextExtMacos},
        tests::{get_unique_name, init_logging},
        TrashContext,
    };
    use serial_test::serial;
    use std::fs::File;

    #[test]
    #[serial]
    fn test_delete_with_ns_file_manager() {
        init_logging();
        let mut trash_ctx = TrashContext::default();
        trash_ctx.set_delete_method(DeleteMethod::NsFileManager);

        let path = get_unique_name();
        File::create(&path).unwrap();
        trash_ctx.delete(&path).unwrap();
        assert!(File::open(&path).is_err());
    }
}
