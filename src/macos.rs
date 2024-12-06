use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use log::trace;
use objc2_foundation::{NSFileManager, NSString, NSURL};

use crate::{into_unknown, Error, TrashContext};

#[derive(Copy, Clone, Debug)]
/// There are 2 ways to trash files: via the ≝Finder app or via the OS NsFileManager call
///
///   | <br>Feature            |≝<br>Finder     |<br>NsFileManager |
///   |:-----------------------|:--------------:|:----------------:|
///   |Undo via "Put back"     | ✓              | ✗                |
///   |Speed                   | ✗<br>Slower    | ✓<br>Faster      |
///   |No sound                | ✗              | ✓                |
///   |No extra permissions    | ✗              | ✓                |
///
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
        match self.platform_specific.delete_method {
            DeleteMethod::Finder => delete_using_finder(&full_paths),
            DeleteMethod::NsFileManager => delete_using_file_mgr(&full_paths),
        }
    }
}

fn delete_using_file_mgr<P: AsRef<Path>>(full_paths: &[P]) -> Result<(), Error> {
    trace!("Starting delete_using_file_mgr");
    let file_mgr = unsafe { NSFileManager::defaultManager() };
    for path in full_paths {
        let path = path.as_ref().as_os_str().as_encoded_bytes();
        let path = match std::str::from_utf8(path) {
            Ok(path_utf8) => NSString::from_str(path_utf8), // utf-8 path, use as is
            Err(_) => NSString::from_str(&percent_encode(path)), // binary path, %-encode it
        };

        trace!("Starting fileURLWithPath");
        let url = unsafe { NSURL::fileURLWithPath(&path) };
        trace!("Finished fileURLWithPath");

        trace!("Calling trashItemAtURL");
        let res = unsafe { file_mgr.trashItemAtURL_resultingItemURL_error(&url, None) };
        trace!("Finished trashItemAtURL");

        if let Err(err) = res {
            return Err(Error::Unknown {
                description: format!("While deleting '{:?}', `trashItemAtURL` failed: {err}", path.as_ref()),
            });
        }
    }
    Ok(())
}

fn delete_using_finder<P: AsRef<Path>>(full_paths: &[P]) -> Result<(), Error> {
    // AppleScript command to move files (or directories) to Trash looks like
    //   osascript -e 'tell application "Finder" to delete { POSIX file "file1", POSIX "file2" }'
    // The `-e` flag is used to execute only one line of AppleScript.
    let mut command = Command::new("osascript");
    let posix_files = full_paths
        .iter()
        .map(|p| {
            let path_b = p.as_ref().as_os_str().as_encoded_bytes();
            match std::str::from_utf8(path_b) {
                Ok(path_utf8) => format!(r#"POSIX file "{}""#,esc_quote(path_utf8)), // utf-8 path, escape \"
                Err(_) => format!(       r#"POSIX file "{}""#,esc_quote(&percent_encode(path_b))), // binary path, %-encode it and escape \"
            }
        })
        .collect::<Vec<String>>()
        .join(", ");
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

/// std's from_utf8_lossy, but non-utf8 byte sequences are %-encoded instead of being replaced by a special symbol.
/// Valid utf8, including `%`, are not escaped.
use std::borrow::Cow;
fn percent_encode(input: &[u8]) -> Cow<'_, str> {
    use percent_encoding::percent_encode_byte as b2pc;

    let mut iter = input.utf8_chunks().peekable();
    if let Some(chunk) = iter.peek() {
        if chunk.invalid().is_empty() {
            return Cow::Borrowed(chunk.valid());
        }
    } else {
        return Cow::Borrowed("");
    };

    let mut res = String::with_capacity(input.len());
    for chunk in iter {
        res.push_str(chunk.valid());
        let invalid = chunk.invalid();
        if !invalid.is_empty() {
            for byte in invalid {
                res.push_str(b2pc(*byte));
            }
        }
    }
    Cow::Owned(res)
}

/// Escapes `"` or `\` with `\` for use in AppleScript text
fn esc_quote(s: &str) -> Cow<'_, str> {
  if s.contains(&['"','\\']) {
    let mut r = String::with_capacity(s.len());
    let chars = s.chars();
    for c in chars { match c {
       '"'
      |'\\' => {r.push('\\');r.push(c);}, // escapes quote/escape char
      _     => {             r.push(c);}, // no escape required
    }};
    Cow::Owned   (r)
  } else {
    Cow::Borrowed(s)
  }
}

#[cfg(test)]
mod tests {
    use crate::{
        macos::{percent_encode, DeleteMethod, TrashContextExtMacos},
        tests::{get_unique_name, init_logging},
        TrashContext,
    };
    use serial_test::serial;
    use std::ffi::OsStr;
    use std::fs::File;
    use std::os::unix::ffi::OsStrExt;
    use std::path::PathBuf;
    use std::process::Command;

    #[test]
    #[serial]
    fn test_delete_with_ns_file_manager() {
        init_logging();
        let mut trash_ctx = TrashContext::default();
        trash_ctx.set_delete_method(DeleteMethod::NsFileManager);

        let path = get_unique_name();
        File::create_new(&path).unwrap();
        trash_ctx.delete(&path).unwrap();
        assert!(File::open(&path).is_err());
    }

    #[test]
    #[serial]
    fn test_delete_binary_path_with_ns_file_manager() {
        let (_cleanup, tmp) = create_hfs_volume().unwrap();
        let parent_fs_supports_binary = tmp.path();

        init_logging();
        let mut trash_ctx = TrashContext::default();
        trash_ctx.set_delete_method(DeleteMethod::NsFileManager);

        let invalid_utf8 = b"\x80"; // lone continuation byte (128) (invalid utf8)
        let mut path_invalid = parent_fs_supports_binary.join(get_unique_name());
        path_invalid.set_extension(OsStr::from_bytes(invalid_utf8)); //...trash-test-111-0.\x80 (not push to avoid fail unexisting dir)

        File::create_new(&path_invalid).unwrap();

        assert!(path_invalid.exists());
        trash_ctx.delete(&path_invalid).unwrap();
        assert!(!path_invalid.exists());
    }

    #[test]
    fn test_path_byte() {
        let invalid_utf8 = b"\x80"; // lone continuation byte (128) (invalid utf8)
        let percent_encoded = "%80"; // valid macOS path in a %-escaped encoding

        let mut expected_path = PathBuf::from(get_unique_name());
        let mut path_with_invalid_utf8 = expected_path.clone();

        path_with_invalid_utf8.push(OsStr::from_bytes(invalid_utf8)); //      trash-test-111-0/\x80
        expected_path.push(percent_encoded); //                    trash-test-111-0/%80

        let actual = percent_encode(&path_with_invalid_utf8.as_os_str().as_encoded_bytes()); // trash-test-111-0/%80
        assert_eq!(std::path::Path::new(actual.as_ref()), expected_path);
    }

    fn create_hfs_volume() -> std::io::Result<(impl Drop, tempfile::TempDir)> {
        let tmp = tempfile::tempdir()?;
        let dmg_file = tmp.path().join("fs.dmg");
        let cleanup = {
            // Create dmg file
            Command::new("hdiutil").args(["create", "-size", "1m", "-fs", "HFS+"]).arg(&dmg_file).status()?;

            // Mount dmg file into temporary location
            Command::new("hdiutil")
                .args(["attach", "-nobrowse", "-mountpoint"])
                .arg(tmp.path())
                .arg(&dmg_file)
                .status()?;

            // Ensure that the mount point is always cleaned up
            defer::defer({
                let mount_point = tmp.path().to_owned();
                move || {
                    Command::new("hdiutil")
                        .arg("detach")
                        .arg(&mount_point)
                        .status()
                        .expect("detach temporary test dmg filesystem successfully");
                }
            })
        };
        Ok((cleanup, tmp))
    }
}
