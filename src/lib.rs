use std::path::Path;

#[cfg(test)]
mod tests;

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod platform;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod platform;

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod platform;

#[derive(Debug)]
pub enum Error {
    Unknown,

    /// Error while canonicalizing path.
    /// `code` contains a raw os error code if accessible.
    CanonicalizePath {
        code: Option<i32>,
    },

    /// Error while performing the remove operation.
    /// `code` contains a raw os error code if accessible.
    Remove {
        code: Option<i32>,
    },
}

/// Removes a single file.
pub fn remove<T: AsRef<Path>>(path: T) -> Result<(), Error> {
    platform::remove(path)
}

/// Removes all files specified by the collection of paths provided as an argument.
pub fn remove_all<I, T>(paths: I) -> Result<(), Error>
where
    I: IntoIterator<Item = T>,
    T: AsRef<Path>,
{
    platform::remove_all(paths)
}

pub fn is_implemented() -> bool {
    platform::is_implemented()
}
