use std::path::Path;

#[cfg(test)]
mod tests;

#[cfg(windows)]
#[path = "windows.rs"]
mod platform;

#[cfg(linux)]
#[path = "linux.rs"]
mod platform;

#[cfg(macos)]
#[path = "macos.rs"]
mod platform;

use platform::*;

#[derive(Debug)]
pub enum Error {
    Unknown,

    /// Error while canonicalizing path
    CanonicalizePath {
        code: Option<i32>,
    },

    /// Error while performing the remove operation
    Remove {
        code: i32,
    },
}

pub fn remove<T: AsRef<Path>>(path: T) -> Result<(), Error> {
    platform_remove(path)
}

pub fn is_implemented() -> bool {
    platform::is_implemented()
}
