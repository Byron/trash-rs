//! This implementation will manage the trash according to the Freedesktop Trash specification,
//! version 1.0 found at https://specifications.freedesktop.org/trash-spec/trashspec-1.0.html
//!
//! If the target system uses a different method for handling trashed items and you would be
//! intrested to use this crate on said system, please open an issue on the github page of `trash`.
//! https://github.com/ArturKovacs/trash
//!

use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono;
use libc;
use scopeguard::defer;

use crate::{Error, ErrorKind, NonStdErrorBox, TrashItem};

static DEFAULT_TRASH: &str = "gio";

/// This is based on the electron library's implementation.
/// See: https://github.com/electron/electron/blob/34c4c8d5088fa183f56baea28809de6f2a427e02/shell/common/platform_util_linux.cc#L96
pub fn remove_all<I, T>(paths: I) -> Result<(), Error>
where
    I: IntoIterator<Item = T>,
    T: AsRef<Path>,
{
    // TODO reimplement this
    //unimplemented!();

    let paths = paths.into_iter();
    let full_paths = paths
        .map(|x| {
            x.as_ref().canonicalize().map_err(|e| {
                Error::new(ErrorKind::CanonicalizePath { original: x.as_ref().into() }, Box::new(e))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let trash = {
        // Determine desktop environment and set accordingly.
        let desktop_env = get_desktop_environment();
        if desktop_env == DesktopEnvironment::Kde4 || desktop_env == DesktopEnvironment::Kde5 {
            "kioclient5"
        } else if desktop_env == DesktopEnvironment::Kde3 {
            "kioclient"
        } else {
            DEFAULT_TRASH
        }
    };

    let mut argv = Vec::<OsString>::with_capacity(full_paths.len() + 2);

    if trash == "kioclient5" || trash == "kioclient" {
        //argv.push(trash.into());
        argv.push("move".into());
        for full_path in full_paths.iter() {
            argv.push(full_path.into());
        }
        argv.push("trash:/".into());
    } else {
        //argv.push_back(ELECTRON_DEFAULT_TRASH);
        argv.push("trash".into());
        for full_path in full_paths.iter() {
            argv.push(full_path.into());
        }
    }

    // Execute command
    let mut command = Command::new(trash);
    command.args(argv);
    let result = command.output().unwrap();
    if !result.status.success() {
        panic!("failed to execute {:?}", command);
    }

    Ok(())
}

pub fn remove<T: AsRef<Path>>(path: T) -> Result<(), Error> {
    remove_all(&[path])
}

pub fn list() -> Result<Vec<TrashItem>, Error> {
    let mut trash_folders = HashSet::new();

    // Get home trash folder and add it to the set of trash folders.
    // It may not exist and that's completely fine as long as there are other trash folders.
    let home_error;
    match home_trash() {
        Ok(home_trash) => {
            trash_folders.insert(home_trash);
            home_error = None;
        }
        Err(e) => {
            home_error = Some(e);
        }
    }

    // Get all mountpoints and attemt to find a trash folder in each adding them to the SET of
    // trash folders when found one.
    let uid = unsafe { libc::getuid() };
    let mount_points = get_mount_points()?;
    for mount in mount_points.into_iter() {
        // See if there's a ".Trash" directory at the mounted location
        let trash_path = mount.mnt_dir.join("/.Trash");
        if trash_path.exists() && trash_path.is_dir() {
            // TODO Report invalidity to the user.
            if folder_validity(&trash_path)? == TrashValidity::Valid {
                let users_trash_path = trash_path.join(uid.to_string());
                if users_trash_path.exists() && trash_path.is_dir() {
                    trash_folders.insert(users_trash_path.into());
                }
            }
        }
        // See if there's a ".Trash-$UID" directory at the mounted location
        let trash_path = mount.mnt_dir.join(format!(".Trash-{}", uid));
        if trash_path.exists() && trash_path.is_dir() {
            trash_folders.insert(trash_path.into());
        }
    }

    if trash_folders.len() == 0 {
        return Err(home_error.unwrap());
    }

    // List all items from the SET of trash folders
    let mut result = Vec::new();
    for folder in trash_folders.into_iter() {
        // List all items from this trash folder

        // Read the info files for every file
        let trash_folder_parent = folder.parent().unwrap();
        let info_folder = folder.join("info");
        let read_dir = std::fs::read_dir(&info_folder).map_err(|e| {
            Error::new(ErrorKind::Filesystem { path: info_folder.clone() }, Box::new(e))
        })?;
        for entry in read_dir {
            let info_entry;
            if let Ok(entry) = entry {
                info_entry = entry;
            } else {
                // Another thread or process may have removed that entry by now
                continue;
            }
            // Entrt should really be an info file but better safe than sorry
            let file_type;
            if let Ok(f_type) = info_entry.file_type() {
                file_type = f_type;
            } else {
                // Another thread or process may have removed that entry by now
                continue;
            }
            if !file_type.is_file() {
                // TODO if we found a folder just hanging out around the infofiles
                // we might want to report it in a warning
                continue;
            }
            let info_path = info_entry.path();
            let info_file;
            if let Ok(file) = File::open(&info_path) {
                info_file = file;
            } else {
                // Another thread or process may have removed that entry by now
                continue;
            }

            let id = info_path.clone().into();
            let mut name = None;
            let mut original_parent: Option<PathBuf> = None;
            let mut time_deleted = None;

            let info_reader = BufReader::new(info_file);
            // Skip 1 because the first line must be "[Trash Info]"
            'info_lines: for line_result in info_reader.lines().skip(1) {
                // Another thread or process may have removed the infofile by now
                let line = if let Ok(line) = line_result {
                    line
                } else {
                    break 'info_lines;
                };
                let mut split = line.split('=');

                // Just unwraping here because the system is assumed to follow the specification.
                let key = split.next().unwrap().trim();
                let value = split.next().unwrap().trim();

                if key == "Path" {
                    let mut value_path = Path::new(value).to_owned();
                    if value_path.is_relative() {
                        value_path = trash_folder_parent.join(value_path);
                    }
                    let full_path_utf8 = PathBuf::from(parse_uri_path(&value_path));
                    name = Some(full_path_utf8.file_name().unwrap().to_str().unwrap().to_owned());
                    let parent = full_path_utf8.parent().unwrap();
                    original_parent = Some(parent.into());
                } else if key == "DeletionDate" {
                    // So there seems to be this funny thing that the freedesktop trash
                    // specification v1.0 has the following statement "The date and time are to be
                    // in the YYYY-MM-DDThh:mm:ss format (see RFC 3339)." Now this is peculiar
                    // because the described format does not conform RFC 3339. But oh well, I'll
                    // just append a 'Z' indicating that the time has no UTC offset and then it will
                    // be conforming.

                    let mut rfc3339_format = value.to_owned();
                    rfc3339_format.push('Z');
                    let date_time = chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                        rfc3339_format.as_str(),
                    )
                    .unwrap();
                    time_deleted = Some(date_time.timestamp());
                }
            }

            // It may be a good idea to assert that these must be Some when the loop successfully
            // read till the end of the file. Because otherwise the environment may not follow the
            // specification or there's a bug in this crate.
            if let Some(name) = name {
                if let Some(original_parent) = original_parent {
                    if let Some(time_deleted) = time_deleted {
                        result.push(TrashItem { id, name, original_parent, time_deleted });
                    }
                }
            }
        }
    }
    Ok(result)
}

pub fn purge_all<I>(items: I) -> Result<(), Error>
where
    I: IntoIterator<Item = TrashItem>,
{
    for item in items.into_iter() {
        // When purging an item the "in-trash" filename must be parsed from the trashinfo filename
        // which is the filename in the `id` field.
        let info_file = &item.id;

        // A bunch of unwraps here. This is fine because if any of these fail that means
        // that either there's a bug in this code or the target system didn't follow
        // the specification.
        let trash_folder = Path::new(info_file).parent().unwrap().parent().unwrap();
        let name_in_trash = Path::new(info_file).file_stem().unwrap();

        let file = trash_folder.join("files").join(&name_in_trash);
        assert!(file.exists());
        if file.is_dir() {
            std::fs::remove_dir_all(&file).map_err(|e| {
                Error::new(ErrorKind::Filesystem { path: file.into() }, Box::new(e))
            })?;
        // TODO Update directory size cache if there's one.
        } else {
            std::fs::remove_file(&file).map_err(|e| {
                Error::new(ErrorKind::Filesystem { path: file.into() }, Box::new(e))
            })?;
        }
        std::fs::remove_file(info_file).map_err(|e| {
            Error::new(ErrorKind::Filesystem { path: info_file.into() }, Box::new(e))
        })?;
    }

    Ok(())
}

pub fn restore_all<I>(items: I) -> Result<(), Error>
where
    I: IntoIterator<Item = TrashItem>,
{
    // Simply read the items' original location from the infofile and attemp to move the items there
    // and delete the infofile if the move operation was sucessful.

    let mut iter = items.into_iter();
    while let Some(item) = iter.next() {
        // The "in-trash" filename must be parsed from the trashinfo filename
        // which is the filename in the `id` field.
        let info_file = &item.id;

        // A bunch of unwraps here. This is fine because if any of these fail that means
        // that either there's a bug in this code or the target system didn't follow
        // the specification.
        let trash_folder = Path::new(info_file).parent().unwrap().parent().unwrap();
        let name_in_trash = Path::new(info_file).file_stem().unwrap();

        let file = trash_folder.join("files").join(&name_in_trash);
        assert!(file.exists());
        // TODO add option to forcefully replace any target at the restore location
        // if it already exists.
        let original_path = item.original_path();
        if original_path.exists() {
            let remaining: Vec<_> = std::iter::once(item).chain(iter).collect();
            return Err(Error::kind_only(ErrorKind::RestoreCollision {
                path: original_path,
                remaining_items: remaining,
            }));
        }
        std::fs::rename(&file, &original_path)
            .map_err(|e| Error::new(ErrorKind::Filesystem { path: file }, Box::new(e)))?;
        std::fs::remove_file(info_file).map_err(|e| {
            Error::new(ErrorKind::Filesystem { path: info_file.into() }, Box::new(e))
        })?;
    }
    Ok(())
}

fn parse_uri_path(absolute_file_path: impl AsRef<Path>) -> String {
    let file_path_chars = absolute_file_path.as_ref().to_str().unwrap().chars();
    let url: String = "file://".chars().chain(file_path_chars).collect();
    return url::Url::parse(&url).unwrap().to_file_path().unwrap().to_str().unwrap().into();
}

#[derive(Eq, PartialEq)]
enum TrashValidity {
    Valid,
    InvalidSymlink,
    InvalidNotSticky,
}

fn folder_validity(path: impl AsRef<Path>) -> Result<TrashValidity, Error> {
    /// Mask for the sticky bit
    /// Taken from: http://man7.org/linux/man-pages/man7/inode.7.html
    const S_ISVTX: u32 = 0x1000;

    let metadata = path.as_ref().symlink_metadata().map_err(|e| {
        Error::new(ErrorKind::Filesystem { path: path.as_ref().into() }, Box::new(e))
    })?;
    if metadata.file_type().is_symlink() {
        return Ok(TrashValidity::InvalidSymlink);
    }
    let mode = metadata.permissions().mode();
    let no_sticky_bit = (mode & S_ISVTX) == 0;
    if no_sticky_bit {
        return Ok(TrashValidity::InvalidNotSticky);
    }
    Ok(TrashValidity::Valid)
}

/// Corresponds to the definition of "home_trash" from
/// https://specifications.freedesktop.org/trash-spec/trashspec-1.0.html
fn home_trash() -> Result<PathBuf, Error> {
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        if data_home.len() > 0 {
            let data_home_path = AsRef::<Path>::as_ref(data_home.as_os_str());
            return Ok(data_home_path.join("Trash").into());
        }
    } else if let Some(home) = std::env::var_os("HOME") {
        if home.len() > 0 {
            let home_path = AsRef::<Path>::as_ref(home.as_os_str());
            return Ok(home_path.join(".local/share/Trash").into());
        }
    }

    panic!("TODO add error kind for when the home trash is not found.");
}

struct MountPoint {
    mnt_dir: PathBuf,
    _mnt_type: String,
    _mnt_fsname: String,
}

fn get_mount_points() -> Result<Vec<MountPoint>, Error> {
    //let file;
    let read_arg = CString::new("r").unwrap();
    let mounts_path = CString::new("/proc/mounts").unwrap();
    let mut file =
        unsafe { libc::fopen(mounts_path.as_c_str().as_ptr(), read_arg.as_c_str().as_ptr()) };
    if file == std::ptr::null_mut() {
        let mtab_path = CString::new("/etc/mtab").unwrap();
        file = unsafe { libc::fopen(mtab_path.as_c_str().as_ptr(), read_arg.as_c_str().as_ptr()) };
    }
    if file == std::ptr::null_mut() {
        // TODO ADD ERROR FOR WHEN NO MONTPOINTS FILE WAS FOUND
        panic!();
    }
    defer! {{ unsafe { libc::fclose(file); } }}
    let mut result = Vec::new();
    loop {
        let mntent = unsafe { libc::getmntent(file) };
        if mntent == std::ptr::null_mut() {
            break;
        }
        let mount_point = unsafe {
            MountPoint {
                mnt_dir: CStr::from_ptr((*mntent).mnt_dir).to_str().unwrap().into(),
                _mnt_fsname: CStr::from_ptr((*mntent).mnt_fsname).to_str().unwrap().into(),
                _mnt_type: CStr::from_ptr((*mntent).mnt_type).to_str().unwrap().into(),
            }
        };
        result.push(mount_point);
    }
    if result.len() == 0 {
        // TODO add Error for when no mountpoints were found
        panic!();
    }
    Ok(result)
}

#[derive(PartialEq)]
enum DesktopEnvironment {
    Other,
    Cinnamon,
    Gnome,
    // KDE3, KDE4 and KDE5 are sufficiently different that we count
    // them as different desktop environments here.
    Kde3,
    Kde4,
    Kde5,
    Pantheon,
    Unity,
    Xfce,
}

fn env_has_var(name: &str) -> bool {
    env::var_os(name).is_some()
}

/// See: https://chromium.googlesource.com/chromium/src/+/dd407d416fa941c04e33d81f2b1d8cab8196b633/base/nix/xdg_util.cc#57
fn get_desktop_environment() -> DesktopEnvironment {
    static KDE_SESSION_ENV_VAR: &str = "KDE_SESSION_VERSION";
    // XDG_CURRENT_DESKTOP is the newest standard circa 2012.
    if let Ok(xdg_current_desktop) = env::var("XDG_CURRENT_DESKTOP") {
        // It could have multiple values separated by colon in priority order.
        for value in xdg_current_desktop.split(":") {
            let value = value.trim();
            if value.len() == 0 {
                continue;
            }
            if value == "Unity" {
                // gnome-fallback sessions set XDG_CURRENT_DESKTOP to Unity
                // DESKTOP_SESSION can be gnome-fallback or gnome-fallback-compiz
                if let Ok(desktop_session) = env::var("DESKTOP_SESSION") {
                    if desktop_session.find("gnome-fallback").is_some() {
                        return DesktopEnvironment::Gnome;
                    }
                }
                return DesktopEnvironment::Unity;
            }
            if value == "GNOME" {
                return DesktopEnvironment::Gnome;
            }
            if value == "X-Cinnamon" {
                return DesktopEnvironment::Cinnamon;
            }
            if value == "KDE" {
                if let Ok(kde_session) = env::var(KDE_SESSION_ENV_VAR) {
                    if kde_session == "5" {
                        return DesktopEnvironment::Kde5;
                    }
                }
                return DesktopEnvironment::Kde4;
            }
            if value == "Pantheon" {
                return DesktopEnvironment::Pantheon;
            }
            if value == "XFCE" {
                return DesktopEnvironment::Xfce;
            }
        }
    }

    // DESKTOP_SESSION was what everyone  used in 2010.
    if let Ok(desktop_session) = env::var("DESKTOP_SESSION") {
        if desktop_session == "gnome" || desktop_session == "mate" {
            return DesktopEnvironment::Gnome;
        }
        if desktop_session == "kde4" || desktop_session == "kde-plasma" {
            return DesktopEnvironment::Kde4;
        }
        if desktop_session == "kde" {
            // This may mean KDE4 on newer systems, so we have to check.
            if env_has_var(KDE_SESSION_ENV_VAR) {
                return DesktopEnvironment::Kde4;
            }
            return DesktopEnvironment::Kde3;
        }
        if desktop_session.find("xfce").is_some() || desktop_session == "xubuntu" {
            return DesktopEnvironment::Xfce;
        }
    }

    // Fall back on some older environment variables.
    // Useful particularly in the DESKTOP_SESSION=default case.
    if env_has_var("GNOME_DESKTOP_SESSION_ID") {
        return DesktopEnvironment::Gnome;
    }
    if env_has_var("KDE_FULL_SESSION") {
        if env_has_var(KDE_SESSION_ENV_VAR) {
            return DesktopEnvironment::Kde4;
        }
        return DesktopEnvironment::Kde3;
    }

    return DesktopEnvironment::Other;
}
