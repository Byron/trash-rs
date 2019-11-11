use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::{Error, ErrorKind, TrashItem};

pub fn remove_all<I, T>(paths: I) -> Result<(), Error>
where
    I: IntoIterator<Item = T>,
    T: AsRef<Path>,
{
    let full_paths = paths
        .into_iter()
        // Convert paths into canonical, absolute forms and collect errors
        .map(|path| {
            path.as_ref().canonicalize().map_err(|e| {
                Error::new(
                    ErrorKind::CanonicalizePath {
                        original: path.as_ref().into(),
                    },
                    Box::new(e),
                )
            })
        })
        // Convert paths into &strs and collect errors
        .map(|path| {
            path.and_then(|p| {
                match p.to_str() {
                    Some(s) => Ok(s.to_owned()),
                    None => Err(Error::kind_only(ErrorKind::ConvertOsString {
                        // `PathBuf`s are stored as `OsString`s internally, so failure to convert
                        // to a slice reduces appropriately.
                        original: p.into(),
                    })),
                }
            })
        })
        .collect::<Result<Vec<String>, Error>>()?;

    // AppleScript command to move files (or directories) to Trash looks like
    //   osascript -e 'tell application "Finder" to delete { POSIX file "file1", POSIX "file2" }'
    // The `-e` flag is used to execute only one line of AppleScript.
    const APPLESCRIPT: &str = "osascript";
    let mut command = Command::new(APPLESCRIPT);
    let posix_files = full_paths
        .into_iter()
        .map(|p| format!("POSIX file \"{}\"", p))
        .collect::<Vec<String>>()
        .join(", ");
    let script = format!(
        "tell application \"Finder\" to delete {{ {} }}",
        posix_files
    );

    let argv: Vec<OsString> = vec!["-e".into(), script.into()];
    command.args(argv);

    // Execute command
    let result = command.output().map_err(|e| {
        Error::new(
            ErrorKind::PlatformApi {
                function_name: APPLESCRIPT,
                code: e.raw_os_error(),
            },
            Box::new(e),
        )
    })?;

    if !result.status.success() {
        return Err(Error::kind_only(ErrorKind::PlatformApi {
            function_name: APPLESCRIPT,
            code: result.status.code(),
        }));
    }

    Ok(())
}

pub fn remove<T: AsRef<Path>>(path: T) -> Result<(), Error> {
    remove_all(&[path])
}

pub fn list() -> Result<Vec<TrashItem>, Error> {
    unimplemented!();
}

pub fn purge_all<I>(_items: I) -> Result<(), Error>
where
    I: IntoIterator<Item = TrashItem>,
{
    unimplemented!();
}

pub fn restore_all<I>(_items: I) -> Result<(), Error> {
    unimplemented!();
}
