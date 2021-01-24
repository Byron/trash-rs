use std::{env::current_dir, error, fmt, path::Path};

#[cfg(test)]
mod tests;

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
#[non_exhaustive]
pub enum Error {
	Unknown,

	/// One of the target items was a root folder.
	/// If a list of items are requested to be removed by a single function call (e.g. `delete_all`)
	/// and this error is returned, then it's guaranteed that none of the items is removed.
	TargetedRoot,

	/// The `target` does not exist or the process has insufficient permissions to access it.
	CouldNotAccess {
		target: String,
	},

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
			Self::TargetedRoot => write!(f, "One of the target items was a root folder"),
			Self::CouldNotAccess {target} => {
				write!(f, "The following does not exist or the process has insufficient permissions to access it: '{}'", target)
			}
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
/// Warning: when a symbolic link is provided to this function the link target will be removed and
/// the link will become dangling. That is if 'sl' refers to 'my-text-file' and `remove("sl")` is
/// called then 'my-text-file' will be removed and 'sl' will be left dangling.
///
/// # Example
///
/// ```
/// #![allow(deprecated)]
/// use std::fs::File;
/// use trash::remove;
/// File::create("remove_me").unwrap();
/// trash::remove("remove_me").unwrap();
/// assert!(File::open("remove_me").is_err());
/// ```
#[deprecated(
	since = "1.2.0",
	note = "Use the `delete` function instead. Be careful with differences in symbolic link resolution."
)]
pub fn remove<T: AsRef<Path>>(path: T) -> Result<(), Error> {
	platform::remove(path)
}

/// Removes all files/directories specified by the collection of paths provided as an argument.
///
/// Warning: when a symbolic link is provided to this function the link target will be removed and
/// the link will become dangling. That is if 'sl' refers to 'my-text-file' and `remove("sl")` is
/// called then 'my-text-file' will be removed and 'sl' will be left dangling.
///
/// # Example
///
/// ```
/// #![allow(deprecated)]
/// use std::fs::File;
/// use trash::remove_all;
/// File::create("remove_me_1").unwrap();
/// File::create("remove_me_2").unwrap();
/// remove_all(&["remove_me_1", "remove_me_2"]).unwrap();
/// assert!(File::open("remove_me_1").is_err());
/// assert!(File::open("remove_me_2").is_err());
/// ```
#[deprecated(
	since = "1.2.0",
	note = "Use the `delete_all` function instead. Be careful with differences in symbolic link resolution."
)]
pub fn remove_all<I, T>(paths: I) -> Result<(), Error>
where
	I: IntoIterator<Item = T>,
	T: AsRef<Path>,
{
	platform::remove_all(paths)
}

/// Removes a single file or directory.
///
/// When a symbolic link is provided to this function, the sybolic link will be removed and the link
/// target will be kept intact.
///
/// # Example
///
/// ```
/// use std::fs::File;
/// use trash::delete;
/// File::create("delete_me").unwrap();
/// trash::delete("delete_me").unwrap();
/// assert!(File::open("delete_me").is_err());
/// ```
pub fn delete<T: AsRef<Path>>(path: T) -> Result<(), Error> {
	delete_all(&[path])
}

/// Removes all files/directories specified by the collection of paths provided as an argument.
///
/// When a symbolic link is provided to this function, the sybolic link will be removed and the link
/// target will be kept intact.
///
/// # Example
///
/// ```
/// use std::fs::File;
/// use trash::delete_all;
/// File::create("delete_me_1").unwrap();
/// File::create("delete_me_2").unwrap();
/// delete_all(&["delete_me_1", "delete_me_2"]).unwrap();
/// assert!(File::open("delete_me_1").is_err());
/// assert!(File::open("delete_me_2").is_err());
/// ```
pub fn delete_all<I, T>(paths: I) -> Result<(), Error>
where
	I: IntoIterator<Item = T>,
	T: AsRef<Path>,
{
	let paths = paths.into_iter();
	let full_paths = paths
		.map(|x| {
			let target_ref = x.as_ref();
			let target = if target_ref.is_relative() {
				let curr_dir = current_dir().map_err(|_| Error::CouldNotAccess {
					target: "[Current working directory]".into(),
				})?;
				curr_dir.join(target_ref)
			} else {
				target_ref.to_owned()
			};
			let parent = target.parent().ok_or(Error::TargetedRoot)?;
			let canonical_parent = parent
				.canonicalize()
				.map_err(|e| Error::CanonicalizePath { code: e.raw_os_error() })?;
			if let Some(file_name) = target.file_name() {
				Ok(canonical_parent.join(file_name))
			} else {
				// `file_name` is none if the path ends with `..`
				Ok(canonical_parent)
			}
		})
		.collect::<Result<Vec<_>, _>>()?;

	platform::remove_all_canonicalized(full_paths)
}

/// Returns true if the functions are implemented on the current platform.
#[deprecated(
	since = "1.0.0",
	note = "This function always returns true. This crate simply does not compile on platforms that are not supported."
)]
pub fn is_implemented() -> bool {
	platform::is_implemented()
}
