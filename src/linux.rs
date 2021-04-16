//! This implementation will manage the trash according to the Freedesktop Trash specification,
//! version 1.0 found at https://specifications.freedesktop.org/trash-spec/trashspec-1.0.html
//!
//! If the target system uses a different method for handling trashed items and you would be
//! intrested to use this crate on said system, please open an issue on the github page of `trash`.
//! https://github.com/ArturKovacs/trash
//!

use std::collections::HashSet;
use std::ffi::{CStr, CString};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use chrono;
use libc;
use scopeguard::defer;

use crate::{Error, ErrorKind, TrashItem};

pub fn remove_all<I, T>(paths: I) -> Result<(), Error>
where
    I: IntoIterator<Item = T>,
    T: AsRef<Path>,
{
    let paths = paths.into_iter();
    let full_paths = paths
        .map(|x| {
            x.as_ref().canonicalize().map_err(|e| {
                Error::new(ErrorKind::CanonicalizePath { original: x.as_ref().into() }, Box::new(e))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let root = Path::new("/");
    let home_trash = home_trash()?;
    let mount_points = get_mount_points()?;
    let uid = unsafe { libc::getuid() };
    for path in full_paths {
        let mut topdir = None;
        for mount_point in mount_points.iter() {
            if mount_point.mnt_dir == root {
                continue;
            }
            if path.starts_with(&mount_point.mnt_dir) {
                topdir = Some(&mount_point.mnt_dir);
                break;
            }
        }
        let topdir = if let Some(t) = topdir { t } else { root };
        if topdir == root {
            // The home trash may not exist yet.
            // Let's just create it in that case.
            if home_trash.exists() == false {
                let info_folder = home_trash.join("info");
                let files_folder = home_trash.join("files");
                std::fs::create_dir_all(&info_folder).map_err(|e| {
                    Error::new(ErrorKind::Filesystem { path: info_folder }, Box::new(e))
                })?;
                std::fs::create_dir_all(&files_folder).map_err(|e| {
                    Error::new(ErrorKind::Filesystem { path: files_folder }, Box::new(e))
                })?;
            }
            move_to_trash(path, &home_trash, topdir)?;
        } else {
            execute_on_mounted_trash_folders(uid, topdir, true, true, |trash_path| {
                move_to_trash(&path, trash_path, topdir)
            })?;
        }
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
        execute_on_mounted_trash_folders(uid, &mount.mnt_dir, false, false, |trash_path| {
            trash_folders.insert(trash_path);
            Ok(())
        })?;
    }
    if trash_folders.len() == 0 {
        // TODO make this a warning
        return Err(home_error.unwrap());
    }
    // List all items from the set of trash folders
    let mut result = Vec::new();
    for folder in trash_folders.iter() {
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
        // Make sure the parent exists so that `create_dir` doesn't faile due to that.
        std::fs::create_dir_all(&item.original_parent).map_err(|e| {
            Error::new(ErrorKind::Filesystem { path: item.original_parent.clone() }, Box::new(e))
        })?;
        let mut collision = false;
        if file.is_dir() {
            // NOTE create_dir_all succeeds when the path already exist but create_dir
            // fails with `std::io::ErrorKind::AlreadyExists`.
            if let Err(e) = std::fs::create_dir(&original_path) {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    collision = true;
                } else {
                    return Err(Error::new(
                        ErrorKind::Filesystem { path: original_path.clone() },
                        Box::new(e),
                    ));
                }
            }
        } else {
            // File or symlink
            if let Err(e) = OpenOptions::new().create_new(true).write(true).open(&original_path) {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    collision = true;
                } else {
                    return Err(Error::new(
                        ErrorKind::Filesystem { path: original_path.clone() },
                        Box::new(e),
                    ));
                }
            }
        }
        if collision {
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

/// According to the specification (see at the top of the file) there can be are two kinds of
/// trash-folders for a mounted drive or partition.
/// 1, .Trash/uid
/// 2, .Trash-uid
///
/// This function executes `op` providing it with a
/// trash-folder path that's associated with the partition mounted at `topdir`.
///
/// Returns Ok(true) if any of the trash folders were found. Returns Ok(false) otherwise.
fn execute_on_mounted_trash_folders<F: FnMut(PathBuf) -> Result<(), Error>>(
    uid: u32,
    topdir: impl AsRef<Path>,
    first_only: bool,
    create_folder: bool,
    mut op: F,
) -> Result<(), Error> {
    let topdir = topdir.as_ref();
    // See if there's a ".Trash" directory at the mounted location
    let trash_path = topdir.join(".Trash");
    if trash_path.exists() && trash_path.is_dir() {
        // TODO Report invalidity to the user.
        if folder_validity(&trash_path)? == TrashValidity::Valid {
            let users_trash_path = trash_path.join(uid.to_string());
            if users_trash_path.exists() && users_trash_path.is_dir() {
                op(users_trash_path)?;
                if first_only {
                    return Ok(());
                }
            }
        }
    }
    // See if there's a ".Trash-$UID" directory at the mounted location
    let trash_path = topdir.join(format!(".Trash-{}", uid));
    let should_execute;
    if !trash_path.exists() || !trash_path.is_dir() {
        if create_folder {
            std::fs::create_dir(&trash_path).map_err(|e| {
                Error::new(ErrorKind::Filesystem { path: trash_path.clone() }, Box::new(e))
            })?;
            should_execute = true;
        } else {
            should_execute = false;
        }
    } else {
        should_execute = true;
    }
    if should_execute {
        op(trash_path)?;
    }
    Ok(())
}

fn move_to_trash(
    src: impl AsRef<Path>,
    trash_folder: impl AsRef<Path>,
    topdir: impl AsRef<Path>,
) -> Result<(), Error> {
    let src = src.as_ref();
    let trash_folder = trash_folder.as_ref();
    let topdir = topdir.as_ref();
    let root = Path::new("/");
    let files_folder = trash_folder.join("files");
    let info_folder = trash_folder.join("info");
    // This kind of validity must only apply ot administrator style trash folders
    // See Trash directories, (1) at https://specifications.freedesktop.org/trash-spec/trashspec-1.0.html
    //assert_eq!(folder_validity(trash_folder)?, TrashValidity::Valid);

    // When trashing a file one must make sure that every trashed item is uniquely named.
    // However the `rename` function -that is used in *nix systems to move files- by default
    // overwrites the destination. Therefore when multiple threads are removing items with identical
    // names, an implementation might accidently overwrite an item that was just put into the trash
    // if it's not careful enough.
    //
    // The strategy here is to use the `create_new` parameter of `OpenOptions` to
    // try creating a placeholder file in the trash but don't do so if one with an identical name
    // already exist. This newly created empty file can then be safely overwritten by the src file
    // using the `rename` function.
    let filename = src.file_name().unwrap();
    let mut appendage = 0;
    loop {
        use std::io;
        appendage += 1;
        let in_trash_name = if appendage > 1 {
            format!("{}.{}", filename.to_str().unwrap(), appendage)
        } else {
            filename.to_str().unwrap().into()
        };
        let info_name = format!("{}.trashinfo", in_trash_name);
        let info_file_path = info_folder.join(&info_name);
        let info_result = OpenOptions::new().create_new(true).write(true).open(&info_file_path);
        match info_result {
            Err(error) => {
                if error.kind() == io::ErrorKind::AlreadyExists {
                    continue;
                } else {
                    return Err(Error::new(
                        ErrorKind::Filesystem { path: info_file_path.into() },
                        Box::new(error),
                    ));
                }
            }
            Ok(mut file) => {
                // Write the info file before actually moving anything
                let now = chrono::Local::now();
                writeln!(file, "[Trash Info]")
                    .and_then(|_| {
                        let absolute_uri = encode_uri_path(src);
                        let topdir_uri = encode_uri_path(topdir);
                        let relative_untrimmed = absolute_uri
                            .chars()
                            .skip(topdir_uri.chars().count())
                            .collect::<String>();
                        let relative_uri = relative_untrimmed.trim_start_matches('/');
                        let path =
                            if topdir == root { absolute_uri.as_str() } else { relative_uri };
                        writeln!(file, "Path={}", path).and_then(|_| {
                            writeln!(file, "DeletionDate={}", now.format("%Y-%m-%dT%H:%M:%S"))
                        })
                    })
                    .map_err(|e| {
                        Error::new(
                            ErrorKind::Filesystem { path: info_file_path.clone() },
                            Box::new(e),
                        )
                    })?;
            }
        }
        let path = files_folder.join(&in_trash_name);
        match move_items_no_replace(src, &path) {
            Err(error) => {
                // Try to delete the info file but in case it fails, we don't care.
                let _ = std::fs::remove_file(info_file_path);
                if error.kind() == io::ErrorKind::AlreadyExists {
                    continue;
                } else {
                    return Err(Error::new(
                        ErrorKind::Filesystem { path: path.into() },
                        Box::new(error),
                    ));
                }
            }
            Ok(_) => {
                // We did it!
                break;
            }
        }
    }

    Ok(())
}

fn execte_src_to_dst_operation<S1, D1>(
    src: S1,
    dst: D1,
    dir: &'static dyn Fn(&Path) -> Result<(), std::io::Error>,
    file: &'static dyn Fn(&Path, &Path) -> Result<(), std::io::Error>,
) -> Result<(), std::io::Error>
where
    S1: AsRef<Path>,
    D1: AsRef<Path>,
{
    let src = src.as_ref();
    let dst = dst.as_ref();

    let metadata = src.symlink_metadata()?;
    if metadata.is_dir() {
        dir(dst)?;
        let dir_entries = std::fs::read_dir(src)?;
        for entry in dir_entries {
            // Forward the error because it's not okay if something is happening
            // to the files while we are trying to move them.
            let entry = entry?;
            let entry_src = entry.path();
            let entry_dst = dst.join(entry.file_name());
            execte_src_to_dst_operation(entry_src, entry_dst, dir, file)?;
        }
    } else {
        // Symlink or file
        file(&src, &dst)?;
    }
    Ok(())
}

/// An error means that a collision was found.
fn move_items_no_replace(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
) -> Result<(), std::io::Error> {
    let src = src.as_ref();
    let dst = dst.as_ref();

    try_creating_placeholders(src, dst)?;

    // All placeholders are in place. LET'S OVERWRITE
    execte_src_to_dst_operation(src, dst, &|_| Ok(()), &|src, dst| std::fs::rename(src, dst))?;

    // Once everything is moved, lets recursively remove the directory
    if src.is_dir() {
        std::fs::remove_dir_all(src)?;
    }
    Ok(())
}

fn try_creating_placeholders(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
) -> Result<(), std::io::Error> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    let metadata = src.symlink_metadata()?;
    if metadata.is_dir() {
        // NOTE create_dir fails if the directory already exists
        std::fs::create_dir(dst)?;
    } else {
        // Symlink or file
        OpenOptions::new().create_new(true).write(true).open(dst)?;
    }
    Ok(())
}

fn parse_uri_path(absolute_file_path: impl AsRef<Path>) -> String {
    let file_path_chars = absolute_file_path.as_ref().to_str().unwrap().chars();
    let url: String = "file://".chars().chain(file_path_chars).collect();
    return url::Url::parse(&url).unwrap().to_file_path().unwrap().to_str().unwrap().into();
}

fn encode_uri_path(absolute_file_path: impl AsRef<Path>) -> String {
    let url = url::Url::from_file_path(absolute_file_path.as_ref()).unwrap();
    url.path().to_owned()
}

#[derive(Eq, PartialEq, Debug)]
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
    }
    if let Some(home) = std::env::var_os("HOME") {
        if home.len() > 0 {
            let home_path = AsRef::<Path>::as_ref(home.as_os_str());
            return Ok(home_path.join(".local/share/Trash").into());
        }
    }
    panic!("Neither the XDG_DATA_HOME nor the HOME environment variable was found");
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
        panic!("Neither '/proc/mounts' nor '/etc/mtab' could be opened.");
    }
    defer! { unsafe { libc::fclose(file); } }
    let mut result = Vec::new();
    loop {
        let mntent = unsafe { libc::getmntent(file) };
        if mntent == std::ptr::null_mut() {
            break;
        }
        let dir = unsafe { CStr::from_ptr((*mntent).mnt_dir).to_str().unwrap() };
        if dir.bytes().len() == 0 {
            continue;
        }
        let mount_point = unsafe {
            MountPoint {
                mnt_dir: dir.into(),
                _mnt_fsname: CStr::from_ptr((*mntent).mnt_fsname).to_str().unwrap().into(),
                _mnt_type: CStr::from_ptr((*mntent).mnt_type).to_str().unwrap().into(),
            }
        };
        result.push(mount_point);
    }
    if result.len() == 0 {
        panic!(
            "A mount points file could be opened but the first call to `getmntent` returned NULL."
        );
    }
    Ok(result)
}
