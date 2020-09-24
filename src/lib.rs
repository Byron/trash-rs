use std::error;
use std::fmt;
use std::path::Path;

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod platform;

#[cfg(all(unix, not(target_os = "macos")))]
#[path = "linux.rs"]
mod platform;

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod platform;

/// Error that might happen during a remove operation.
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

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Unknown => write!(f, "Unknown error"),
			Self::CanonicalizePath { code } => {
				let code_str = match code {
					Some(i) => format!("Error code was {}", i),
					None => "No error code was available".into(),
				};
				write!(f, "Error while canonicalizing path. {}", code_str)
			}
			Self::Remove { code } => {
				let code_str = match code {
					Some(i) => format!("Error code was {}", i),
					None => "No error code was available".into(),
				};
				write!(f, "Error while performing the remove operation. {}", code_str)
			}
		}
	}
}

impl error::Error for Error {}

/// Removes a single file or directory.
///
/// # Example
///
/// ```
/// extern crate trash;
/// use std::fs::File;
/// use trash::remove;
/// File::create("remove_me").unwrap();
/// trash::remove("remove_me").unwrap();
/// assert!(File::open("remove_me").is_err());
/// ```
pub fn remove<T: AsRef<Path>>(path: T) -> Result<(), Error> {
	platform::remove(path)
}

/// Removes all files/directories specified by the collection of paths provided as an argument.
///
/// # Example
///
/// ```
/// extern crate trash;
/// use std::fs::File;
/// use trash::remove_all;
/// File::create("remove_me_1").unwrap();
/// File::create("remove_me_2").unwrap();
/// remove_all(&["remove_me_1", "remove_me_2"]).unwrap();
/// assert!(File::open("remove_me_1").is_err());
/// assert!(File::open("remove_me_2").is_err());
/// ```
pub fn remove_all<I, T>(paths: I) -> Result<(), Error>
where
	I: IntoIterator<Item = T>,
	T: AsRef<Path>,
{
	platform::remove_all(paths)
}

/// Returns true if the functions are implemented on the current platform.
#[deprecated(
	since = "1.0.0",
	note = "This function always returns true. This crate simply does not compile on platforms that are not supported."
)]
pub fn is_implemented() -> bool {
	platform::is_implemented()
}
