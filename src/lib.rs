use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::hash::{Hash, Hasher};

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

/// Error that might happen during a trash operation.
#[derive(Debug)]
pub enum Error {
    Unknown,

    /// Any error that might happen during a direct call to a platform specific API.
    ///
    /// `function_name`: the name of the function during which the error occured.
    /// `code`: An error code that the function provided or was obtained otherwise.
    ///
    /// On Windows the `code` will contain the HRESULT that the function returned or that was
    /// obtained with `HRESULT_FROM_WIN32(GetLastError())`
    PlatformApi {
        function_name: String,
        code: Option<i32>,
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

    /// Error while converting an OsString to a String.
    /// `original` is the string that was attempted to be converted.
    ConvertOsString {
        original: OsString,
    },
}

/// This struct holds information about a single item within the trash.
///
/// Some functions associated with this struct are defined in the `TrahsItemPlatformDep` trait.
/// That trait is implemented for `TrashItem` by each platform specific source file individually.
///
/// A trahs item can be a file or folder or any other object that the target operating system
/// allows to put into the trash.
#[derive(Debug)]
pub struct TrashItem {
    /// A system specific identifier of the item in the trash.
    ///
    /// On Windows it is the string returned by `IShellFolder::GetDisplayNameOf` with the
    /// `SHGDN_FORPARSING` flag.
    ///
    /// On Linux ...
    ///
    /// On MacOS ...
    pub id: OsString,

    /// The name of the item. For example if the folder '/home/user/New Folder' was deleted,
    /// its `name` is 'New Folder'
    pub name: String,

    /// The path to the parent folder of this item before it was put inside the trash.
    /// For example if the folder '/home/user/New Folder' is in the trash, its `original_parent`
    /// is '/home/user'.
    /// 
    /// To get the full path to the file in its original location use the `original_path`
    /// function.
    pub original_parent: PathBuf,

    /// The date and time in UNIX Epoch time when the item was put into the trash.
    pub time_deleted: i64,
}
/// Platform independent functions of `TrashItem`.
///
/// See `TrahsItemPlatformDep` for platform dependent functions.
impl TrashItem {
    /// Joins the `original_parent` and `name` fields to obtain the full path to the original file.
    pub fn original_path(&self) -> PathBuf {
        self.original_parent.join(&self.name)
    }
}
impl PartialEq for TrashItem {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for TrashItem {}
impl Hash for TrashItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// Returns all `TrashItem`s that are currently in the trash.
pub fn list() -> Result<Vec<TrashItem>, Error> {
    platform::list()
}

/// Deletes all the provided items permanently.
///
/// This function consumes the provided `TrashItem`s.
pub fn purge_all<I>(items: I) -> Result<(), Error>
where
    I: IntoIterator<Item = TrashItem>,
{
    platform::purge_all(items)
}

/// Restores all the provided items to their original location.
///
/// This function consumes the provided `TrashItem`s.
pub fn restore_all<I>(items: I) -> Result<(), Error>
where
    I: IntoIterator<Item = TrashItem>,
{
    platform::restore_all(items)
}

/// This trait lists all the `TrashItem` related functions that have a platform dependent
/// implementation
trait TrahsItemPlatformDep {
    /// Permanently delete the item.
    fn purge(self) -> Result<(), ()>;

    /// Restore the item from the trash to its original location.
    fn restore(self) -> Result<(), ()>;
}

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
