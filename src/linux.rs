use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ffi::{CStr, CString};
use std::collections::HashSet;
use std::fs::{File, Permissions, Metadata};
use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;

use chrono;
use libc;
use scopeguard::defer;

use crate::{Error, ErrorKind, TrashItem};

mod uri_path;

static DEFAULT_TRASH: &str = "gio";

pub fn is_implemented() -> bool {
    true
}

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
                Error::new(ErrorKind::CanonicalizePath {original: x.as_ref().into()}, Box::new(e))
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
        },
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
        let info_folder = folder.join("info");
        let read_dir = std::fs::read_dir(&info_folder).map_err(|e| {
            Error::new(ErrorKind::Filesystem { path: info_folder.clone() }, Box::new(e))
        })?;
        for entry in read_dir {
            let info_entry = entry.map_err(|e| {
                Error::new(ErrorKind::Filesystem { path: info_folder.clone() }, Box::new(e))
            })?;
            // Entrt should really be an info file but better safe than sorry
            let file_type = info_entry.file_type().map_err(|e| {
                Error::new(ErrorKind::Filesystem { path: info_entry.path() }, Box::new(e))
            })?;
            if !file_type.is_file() { continue; }
            let info_path = info_entry.path();
            let info_file = File::open(&info_path).map_err(|e| {
                Error::new(ErrorKind::Filesystem { path: info_path.clone() }, Box::new(e))
            })?;

            let id = info_path.clone().into();
            let mut name = None;
            let mut original_parent: Option<PathBuf> = None;
            let mut time_deleted = None;
            
            let info_reader = BufReader::new(info_file);
            // Skip 1 because the first line must be "[Trash Info]"
            for line in info_reader.lines().skip(1) {
                let line = line.map_err(|e| {
                    Error::new(ErrorKind::Filesystem { path: info_path.clone() }, Box::new(e))
                })?;
                let mut split = line.split('=');

                // Just unwraping here because the system is assumed to follow the specification.
                let key = split.next().unwrap().trim();
                let value = split.next().unwrap().trim();

                if key == "Path" {
                    let full_path_string = parse_uri_path(value);
                    let full_path = Path::new(&full_path_string);
                    name = Some(full_path.file_name().unwrap().to_str().unwrap().to_owned());
                    let parent = full_path.parent().unwrap();
                    if full_path.is_absolute() {
                        original_parent = Some(parent.into());
                    } else {
                        original_parent = Some(folder.join(parent).into());
                    }
                } else if key == "DeletionDate" {
                    // So there seems to be this funny thing that the freedesktop trash
                    // specification v1.0 has the following statement "The date and time are to be
                    // in the YYYY-MM-DDThh:mm:ss format (see RFC 3339)." Now this is peculiar
                    // because the described format does not conform RFC 3339. But oh well, I'll
                    // just append a 'Z' indicating that the time has no UTC offset and then it will
                    // be conforming.
                    
                    let mut rfc3339_format = value.to_owned();
                    rfc3339_format.push('Z');
                    let date_time = chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(rfc3339_format.as_str()).unwrap();
                    time_deleted = Some(date_time.timestamp());
                }
            }
            result.push(TrashItem {
                id,
                name: name.unwrap(),
                original_parent: original_parent.unwrap(),
                time_deleted: time_deleted.unwrap(),
            });
        }
    }
    Ok(result)
}

pub fn purge_all<I>(items: I) -> Result<(), Error>
where
    I: IntoIterator<Item = TrashItem>,
{
    Err(Error::kind_only(ErrorKind::PlatformApi {
        function_name: "I lied. This is not a platform api error, but a NOT IMPLEMENTED error.",
        code: None,
    }))
}

pub fn restore_all<I>(items: I) -> Result<(), Error>
where
    I: IntoIterator<Item = TrashItem>,
{
    unimplemented!();
}

fn parse_uri_path(uri: impl AsRef<str>) -> String {
    // TODO have some fun with implementing this, after you got the groceries
    use std::convert::TryFrom;
    let mut path = uri_path::Path::try_from(uri.as_ref()).unwrap();
    //path.normalize(true);
    let uri_encoded = path.to_string();
    let mut result_bytes = Vec::<u8>::with_capacity(uri_encoded.len());
    let mut bytes = uri_encoded.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let high_digit = bytes.next().unwrap();
            let low_digit = bytes.next().unwrap();
            let high_value = to_decimal_value(high_digit as char);
            let low_value = to_decimal_value(low_digit as char);
            let value = 0x10 * high_value + low_value;
            result_bytes.push(value);
        } else {
            result_bytes.push(byte);
        }
    }
    String::from_utf8(result_bytes).unwrap()
}

mod test_parse_uri {
    #[test]
    fn uri() {
        let result = super::parse_uri_path("/path/to/I%E2%9D%A4%EF%B8%8FURIs");
        assert_eq!(result, "/path/to/I❤️URIs");
    }
}

fn to_decimal_value(hex_digit: char) -> u8 {
    let hex_digit = hex_digit.to_lowercase().next().unwrap() as u8;
    if hex_digit >= b'0' && hex_digit <= b'9' {
        (hex_digit - b'0') as u8
    } else {
        (hex_digit - b'a') as u8 + 10
    }
}

#[derive(Eq, PartialEq)]
enum TrashValidity { Valid, InvalidSymlink, InvalidNotSticky }

fn folder_validity(path: impl AsRef<Path>) -> Result<TrashValidity, Error> {
    /// Mask for the sticky bit
    /// Taken from: http://man7.org/linux/man-pages/man7/inode.7.html
    const S_ISVTX: u32 = 0x1000;

    let metadata = path.as_ref().symlink_metadata().map_err(|e| {
        Error::new(ErrorKind::Filesystem {path: path.as_ref().into()}, Box::new(e))
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
    mnt_type: String,
    mnt_fsname: String,
}

fn get_mount_points() -> Result<Vec<MountPoint>, Error> {
    //let file;
    let read_arg = CString::new("r").unwrap();
    let mounts_path = CString::new("/proc/mounts").unwrap();
    let mut file = unsafe {
        libc::fopen(mounts_path.as_c_str().as_ptr(), read_arg.as_c_str().as_ptr())
    };
    if file == std::ptr::null_mut() {
        let mtab_path = CString::new("/etc/mtab").unwrap();
        file = unsafe {
            libc::fopen(mtab_path.as_c_str().as_ptr(), read_arg.as_c_str().as_ptr())
        };
    }
    if file == std::ptr::null_mut() {
        // TODO ADD ERROR FOR WHEN NO MONTPOINTS FILE WAS FOUND
        panic!();
    }
    defer!{{ unsafe { libc::fclose(file); } }}
    let mut result = Vec::new();
    loop {
        let mntent = unsafe { libc::getmntent(file) };
        if mntent == std::ptr::null_mut() {
            break;
        }
        let mount_point = unsafe{ MountPoint {
            mnt_dir: CStr::from_ptr((*mntent).mnt_dir).to_str().unwrap().into(),
            mnt_fsname: CStr::from_ptr((*mntent).mnt_fsname).to_str().unwrap().into(),
            mnt_type: CStr::from_ptr((*mntent).mnt_type).to_str().unwrap().into(),
        }};
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
