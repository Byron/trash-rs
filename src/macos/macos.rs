use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use log::{trace,warn};
use objc2_foundation::{NSFileManager, NSString, NSURL};
use objc2::rc::Retained;

use crate::{into_unknown, Error, TrashContext, TrashItem};

#[derive(Copy, Clone, Debug)]
/// There are 2 ways to trash files: via the ≝Finder app or via the OS NsFileManager call
///
///   | <br>Feature            |≝<br>Finder     |<br>NsFileManager |
///   |:-----------------------|:--------------:|:----------------:|
///   |Undo via "Put back"     | ✓              | ✗                |
///   |Get trashed paths       | ✗              | ✓                |
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
    /// - Allows getting the paths to the trashed items
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
    pub(crate) fn delete_all_canonicalized(&self, full_paths: Vec<PathBuf>) -> Result<Option<Vec<TrashItem>>, Error> {
        match self.platform_specific.delete_method {
            DeleteMethod::Finder =>  delete_using_finder(&full_paths),
            DeleteMethod::NsFileManager =>  delete_using_file_mgr(&full_paths),
        }
    }
}

fn delete_using_file_mgr<P: AsRef<Path>>(full_paths: &[P]) -> Result<Option<Vec<TrashItem>>, Error> {
    trace!("Starting delete_using_file_mgr");
    let file_mgr = unsafe { NSFileManager::defaultManager() };
    let mut items = Vec::with_capacity(full_paths.len());
    for path in full_paths {
        let path_r = path.as_ref();
        let path = path_r.as_os_str().as_encoded_bytes();
        let path = match std::str::from_utf8(path) {
            Ok(path_utf8) => NSString::from_str(path_utf8), // utf-8 path, use as is
            Err(_) => NSString::from_str(&percent_encode(path)), // binary path, %-encode it
        };

        trace!("Starting fileURLWithPath");
        let url = unsafe { NSURL::fileURLWithPath(&path) };
        trace!("Finished fileURLWithPath");

        trace!("Calling trashItemAtURL");
        let mut out_res_nsurl: Option<Retained<NSURL>> = None;
        let res = unsafe { file_mgr.trashItemAtURL_resultingItemURL_error(&url, Some(&mut out_res_nsurl)) };
        trace!("Finished trashItemAtURL");

        if let Err(err) = res {
            return Err(Error::Unknown {
                description: format!("While deleting '{:?}', `trashItemAtURL` failed: {err}", path),
            });
        } else {
            if let Some(out_nsurl) = out_res_nsurl {
                let mut time_deleted = -1;
                #[cfg(    feature = "chrono") ] {let now = chrono::Local::now(); time_deleted = now.timestamp();}
                #[cfg(not(feature = "chrono"))] {                                time_deleted = -1;}
                if let Some(nspath) = unsafe {out_nsurl.path()} { // Option<Retained<NSString>>
                    items.push(TrashItem {
                        id             : nspath.to_string().into(),
                        name           : path_r.file_name().expect("Item to be trashed should have a name"  ).into(),
                        original_parent: path_r.parent   ().expect("Item to be trashed should have a parent").to_path_buf(),
                        time_deleted,
                    });
                } else {warn!("OS did not return path string from the URL of the trashed item '{:?}', originally located at: '{:?}'", out_nsurl, path);}
            }     else {warn!("OS did not return a path to the trashed file, originally located at: '{:?}'"                                    , path);}
        }
    }
    Ok(Some(items))
}

fn delete_using_finder<P: AsRef<Path>>(full_paths: &[P]) -> Result<Option<Vec<TrashItem>>, Error> {
    // AppleScript command to move files (or directories) to Trash looks like
    //   osascript -e 'tell application "Finder" to delete { POSIX file "file1", POSIX "file2" }'
    // The `-e` flag is used to execute only one line of AppleScript.
    let mut command = Command::new("osascript");
    let posix_files = full_paths
        .iter()
        .map(|p| {
            let path_b = p.as_ref().as_os_str().as_encoded_bytes();
            match std::str::from_utf8(path_b) {
                Ok(path_utf8) => format!(r#"POSIX file "{}""#, esc_quote(path_utf8)), // utf-8 path, escape \"
                Err(_) => format!(r#"POSIX file "{}""#, esc_quote(&percent_encode(path_b))), // binary path, %-encode it and escape \"
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
    Ok(None)
}

/// std's from_utf8_lossy, but non-utf8 byte sequences are %-encoded instead of being replaced by a special symbol.
/// Valid utf8, including `%`, are not escaped.
use std::borrow::Cow;
pub(crate) fn percent_encode(input: &[u8]) -> Cow<'_, str> {
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
pub(crate) fn esc_quote(s: &str) -> Cow<'_, str> {
    if s.contains(['"', '\\']) {
        let mut r = String::with_capacity(s.len());
        let chars = s.chars();
        for c in chars {
            match c {
                '"' | '\\' => {
                    r.push('\\');
                    r.push(c);
                } // escapes quote/escape char
                _ => {
                    r.push(c);
                } // no escape required
            }
        }
        Cow::Owned(r)
    } else {
        Cow::Borrowed(s)
    }
}

#[cfg(test)]
mod test_macos;
