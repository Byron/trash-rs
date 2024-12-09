use crate::into_unknown;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use log::{trace, warn};
use objc2::rc::Retained;
use objc2_foundation::{NSFileManager, NSString, NSURL};

use crate::{Error, TrashContext, TrashItem};

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

#[derive(Copy, Clone, Debug)]
/// There are 2 ways to ask Finder to trash files: ≝1. by calling the `osascript` binary or 2. calling directly into the `OSAKit` Framework.
/// The `OSAKit` method should be faster, but it MUST be run on the main thread, otherwise it can fail, stalling until the default 2 min
/// timeout expires.
///
pub enum ScriptMethod {
    /// Spawn a process calling the standalone `osascript` binary to run AppleScript. Slower, but more reliable.
    ///
    /// This is the default.
    Cli,

    /// Call into `OSAKit` directly via ObjC-bindings. Faster, but MUST be run on the main thread, or it can fail, stalling for 2 min.
    Osakit,
}
impl ScriptMethod {
    /// Returns `ScriptMethod::Cli`
    pub const fn new() -> Self {
        ScriptMethod::Cli
    }
}
impl Default for ScriptMethod {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Default, Debug)]
pub struct PlatformTrashContext {
    delete_method: DeleteMethod,
    script_method: ScriptMethod,
}
impl PlatformTrashContext {
    pub const fn new() -> Self {
        Self { delete_method: DeleteMethod::new(), script_method: ScriptMethod::new() }
    }
}
pub trait TrashContextExtMacos {
    fn set_delete_method(&mut self, method: DeleteMethod);
    fn delete_method(&self) -> DeleteMethod;
    fn set_script_method(&mut self, method: ScriptMethod);
    fn script_method(&self) -> ScriptMethod;
}
impl TrashContextExtMacos for TrashContext {
    fn set_delete_method(&mut self, method: DeleteMethod) {
        self.platform_specific.delete_method = method;
    }
    fn delete_method(&self) -> DeleteMethod {
        self.platform_specific.delete_method
    }
    fn set_script_method(&mut self, method: ScriptMethod) {
        self.platform_specific.script_method = method;
    }
    fn script_method(&self) -> ScriptMethod {
        self.platform_specific.script_method
    }
}
impl TrashContext {
    pub(crate) fn delete_all_canonicalized(
        &self,
        full_paths: Vec<PathBuf>,
        with_info: bool,
    ) -> Result<Option<Vec<TrashItem>>, Error> {
        match self.platform_specific.delete_method {
            DeleteMethod::Finder => match self.platform_specific.script_method {
                ScriptMethod::Cli => delete_using_finder(&full_paths, with_info, true),
                ScriptMethod::Osakit => delete_using_finder(&full_paths, with_info, false),
            },
            DeleteMethod::NsFileManager => delete_using_file_mgr(&full_paths, with_info),
        }
    }
}

fn delete_using_file_mgr<P: AsRef<Path>>(full_paths: &[P], with_info: bool) -> Result<Option<Vec<TrashItem>>, Error> {
    trace!("Starting delete_using_file_mgr");
    let file_mgr = unsafe { NSFileManager::defaultManager() };
    let mut items = if with_info { Vec::with_capacity(full_paths.len()) } else { vec![] };
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
        let res = if with_info {
            unsafe { file_mgr.trashItemAtURL_resultingItemURL_error(&url, Some(&mut out_res_nsurl)) }
        } else {
            unsafe { file_mgr.trashItemAtURL_resultingItemURL_error(&url, None) }
        };
        trace!("Finished trashItemAtURL");

        if let Err(err) = res {
            return Err(Error::Unknown {
                description: format!("While deleting '{:?}', `trashItemAtURL` failed: {err}", path),
            });
        } else if with_info {
            if let Some(out_nsurl) = out_res_nsurl {
                #[allow(unused_assignments)]
                let mut time_deleted = -1;
                #[cfg(feature = "chrono")]
                {
                    let now = chrono::Local::now();
                    time_deleted = now.timestamp();
                }
                #[cfg(not(feature = "chrono"))]
                {
                    time_deleted = -1;
                }
                if let Some(nspath) = unsafe { out_nsurl.path() } {
                    // Option<Retained<NSString>>
                    items.push(TrashItem {
                        id: nspath.to_string().into(),
                        name: path_r.file_name().expect("Item to be trashed should have a name").into(),
                        original_parent: path_r
                            .parent()
                            .expect("Item to be trashed should have a parent")
                            .to_path_buf(),
                        time_deleted,
                    });
                } else {
                    warn!("OS did not return path string from the URL of the trashed item '{:?}', originally located at: '{:?}'", out_nsurl, path);
                }
            } else {
                warn!("OS did not return a path to the trashed file, originally located at: '{:?}'", path);
            }
        }
    }
    if with_info {
        Ok(Some(items))
    } else {
        Ok(None)
    }
}

fn delete_using_finder<P: AsRef<Path> + std::fmt::Debug>(
    full_paths: &[P],
    with_info: bool,
    as_cli: bool,
) -> Result<Option<Vec<TrashItem>>, Error> {
    // TODO: should we convert to trashing item by item instead of in batches to have a guaranteed match of input to output?
    // which method is faster?
    // what about with a lot of items? will a huge script combining all paths still work?
    let mut items: Vec<TrashItem> = if with_info { Vec::with_capacity(full_paths.len()) } else { vec![] };
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

    const LIST_SEP: &str = " /// ";
    let script_text = if with_info {
        if as_cli {
            // since paths can have any char, use " /// " triple path separator for parsing ouput of list paths
            format!(
                r#"
            tell application "Finder"
                set Trash_items to delete {{ {posix_files} }}
            end tell
            if (class of Trash_items) is not list then -- if only 1 file is deleted, returns a file, not a list
                return                   (POSIX path of (Trash_items as alias))
            end if
            repeat with aFile in Trash_items -- Finder reference
                set contents of aFile to (POSIX path of (aFile as alias)) -- can't get paths of Finder reference, coersion to alias needed
            end repeat
            set text item delimiters to "{LIST_SEP}" -- hopefully no legal path can have this
            return Trash_items as text -- coersion to text forces item delimiters for lists
            "#
            )
        } else {
            format!(
                r#"
            tell application "Finder"
                set Trash_items to delete {{ {posix_files} }}
            end tell
            if (class of Trash_items) is not list then -- if only 1 file is deleted, returns a file, not a list
                return                   (POSIX path of (Trash_items as alias))
            end if
            repeat with aFile in Trash_items -- Finder reference
                set contents of aFile to (POSIX path of (aFile as alias)) -- can't get paths of Finder reference, coersion to alias needed
            end repeat
            return Trash_items
            "#
            )
        }
    } else {
        // no ouput parsing required, so script is the same for Cli and Osakit
        format!(
            r#"tell application "Finder" to delete {{ {posix_files} }}
                return "" "#
        )
    };
    use osakit::{Language, Script};
    if as_cli {
        let mut command = Command::new("osascript");
        let argv: Vec<OsString> = vec!["-e".into(), script_text.into()];
        command.args(argv);

        // Execute command
        let result = command.output().map_err(into_unknown)?;
        if result.status.success() {
            if with_info {
                // parse stdout into a list of paths and convert to TrashItems
                #[allow(unused_assignments)]
                let mut time_deleted = -1;
                #[cfg(feature = "chrono")]
                {
                    let now = chrono::Local::now();
                    time_deleted = now.timestamp();
                }
                #[cfg(not(feature = "chrono"))]
                {
                    time_deleted = -1;
                }
                let stdout = String::from_utf8_lossy(&result.stdout); // Finder's return paths should be utf8 (%-encoded?)?
                let file_list = stdout.strip_suffix("\n").unwrap_or(&stdout).split(LIST_SEP).collect::<Vec<_>>();

                if !file_list.is_empty() {
                    let len_match = full_paths.len() == file_list.len();
                    if !len_match {
                        warn!("AppleScript returned a list of trashed paths len {} ≠ {} expected items sent to be trashed, so trashed items will have empty names/original parents as we can't be certain which trash path matches which trashed item",full_paths.len(),file_list.len());
                    }
                    for (i, file_path) in file_list.iter().enumerate() {
                        let path_r = if len_match { full_paths[i].as_ref() } else { Path::new("") };
                        items.push(TrashItem {
                            id: file_path.into(),
                            name: if len_match {
                                path_r.file_name().expect("Item to be trashed should have a name").into()
                            } else {
                                "".into()
                            },
                            original_parent: if len_match {
                                path_r.parent().expect("Item to be trashed should have a parent").to_path_buf()
                            } else {
                                "".into()
                            },
                            time_deleted,
                        });
                    }
                    return Ok(Some(items));
                } else {
                    let ss = if full_paths.len() > 1 { "s" } else { "" };
                    warn!("AppleScript did not return a list of path{} to the trashed file{}, originally located at: {:?}",&ss,&ss,&full_paths);
                }
            }
        } else {
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
    } else {
        // use Osakit
        let mut script = Script::new_from_source(Language::AppleScript, &script_text);

        // Compile and Execute script
        match script.compile() {
            Ok(_) => match script.execute() {
                Ok(res) => {
                    if with_info {
                        #[allow(unused_assignments)]
                        let mut time_deleted = -1;
                        #[cfg(feature = "chrono")]
                        {
                            let now = chrono::Local::now();
                            time_deleted = now.timestamp();
                        }
                        #[cfg(not(feature = "chrono"))]
                        {
                            time_deleted = -1;
                        }
                        let res_arr = if let Some(file_path) = res.as_str() {
                            // convert a single value into an array for ease of handling later
                            vec![file_path].into()
                        } else {
                            res
                        };
                        if let Some(file_list) = res_arr.as_array() {
                            let len_match = full_paths.len() == file_list.len();
                            if !len_match {
                                warn!("AppleScript returned a list of trashed paths len {} ≠ {} expected items sent to be trashed, so trashed items will have empty names/original parents as we can't be certain which trash path matches which trashed item",full_paths.len(),file_list.len());
                            }
                            for (i, posix_path) in file_list.iter().enumerate() {
                                if let Some(file_path) = posix_path.as_str() {
                                    // Finder's return paths should be utf8 (%-encoded?)?
                                    //let p=PathBuf::from(file_path);
                                    //println!("✓converted posix_path:{}
                                    //        \nexists {}           {:?}", posix_path, p.exists(),p);
                                    let path_r = if len_match { full_paths[i].as_ref() } else { Path::new("") };
                                    items.push(TrashItem {
                                        id: file_path.into(),
                                        name: if len_match {
                                            path_r.file_name().expect("Item to be trashed should have a name").into()
                                        } else {
                                            "".into()
                                        },
                                        original_parent: if len_match {
                                            path_r
                                                .parent()
                                                .expect("Item to be trashed should have a parent")
                                                .to_path_buf()
                                        } else {
                                            "".into()
                                        },
                                        time_deleted,
                                    });
                                } else {
                                    warn!(
                                        "Failed to parse AppleScript's returned path to the trashed file: {:?}",
                                        &posix_path
                                    );
                                }
                            }
                            return Ok(Some(items));
                        } else {
                            let ss = if full_paths.len() > 1 { "s" } else { "" };
                            warn!("AppleScript did not return a list of path{} to the trashed file{}, originally located at: {:?}",&ss,&ss,&full_paths);
                        }
                    }
                }
                Err(e) => {
                    return Err(Error::Unknown { description: format!("The AppleScript failed with error: {}", e) })
                }
            },
            Err(e) => {
                return Err(Error::Unknown {
                    description: format!("The AppleScript failed to compile with error: {}", e),
                })
            }
        }
    }
    Ok(None)
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
mod tests;
