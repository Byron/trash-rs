use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use crate::{into_unknown, Error};

pub fn delete_all_canonicalized(full_paths: Vec<PathBuf>) -> Result<(), Error> {
    let full_paths = full_paths.into_iter().map(to_string).collect::<Result<Vec<_>, _>>()?;
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

// pub fn remove_all<I, T>(paths: I) -> Result<(), Error>
// where
//     I: IntoIterator<Item = T>,
//     T: AsRef<Path>,
// {
//     let full_paths = paths
//         .into_iter()
//         // Convert paths into canonical, absolute forms and collect errors
//         .map(|path| {
//             path.as_ref().canonicalize().map_err(|e| {
//                 Error::new(
//                     ErrorKind::CanonicalizePath {
//                         original: path.as_ref().into(),
//                     },
//                     Box::new(e),
//                 )
//             })
//         })
//         .collect::<Result<Vec<_>, Error>>()?;

//     remove_all_canonicalized(full_paths)
// }

// pub fn remove<T: AsRef<Path>>(path: T) -> Result<(), Error> {
//     remove_all(&[path])
// }

fn to_string<T: Into<OsString>>(str_in: T) -> Result<String, Error> {
    let os_string = str_in.into();
    let s = os_string.to_str();
    match s {
        Some(s) => Ok(s.to_owned()),
        None => {
            std::mem::drop(s);
            Err(Error::ConvertOsString { original: os_string })
        }
    }
}
