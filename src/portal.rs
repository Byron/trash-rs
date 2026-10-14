use std::{
    ffi::OsStr,
    io,
    os::fd::OwnedFd,
    path::{Component, Path, PathBuf},
};

use once_cell::sync::OnceCell;
use rustix::fs::{fstat, open, openat, statat, AtFlags, FileType, Mode, OFlags, Stat};
use rustix::io::Errno;
use zbus::proxy;

use crate::{Error, TrashContext};

#[proxy(
    interface = "org.freedesktop.portal.Trash",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop",
    gen_async = false
)]
trait Trash {
    #[zbus(name = "TrashFile")]
    fn trash_file(&self, fd: zbus::zvariant::Fd<'_>) -> zbus::Result<u32>;
}

#[derive(Clone, Default, Debug)]
pub struct PlatformTrashContext {
    conn: OnceCell<zbus::blocking::Connection>,
    proxy: OnceCell<TrashProxy<'static>>,
}

impl PlatformTrashContext {
    pub const fn new() -> Self {
        Self { conn: OnceCell::new(), proxy: OnceCell::new() }
    }

    fn connection(&self) -> Result<&zbus::blocking::Connection, Error> {
        self.conn.get_or_try_init(|| {
            zbus::blocking::Connection::session().map_err(|err| Error::Portal { status_code: None, source: Some(err) })
        })
    }

    fn proxy(&self) -> Result<&TrashProxy<'_>, Error> {
        let conn = self.connection()?;

        self.proxy.get_or_try_init(|| {
            TrashProxy::new(conn).map_err(|err| Error::Portal { status_code: None, source: Some(err) })
        })
    }
}

impl TrashContext {
    pub(crate) fn delete_all_canonicalized(&self, full_paths: Vec<PathBuf>) -> Result<(), Error> {
        let proxy = self.platform_specific.proxy()?;

        for path in full_paths {
            let fd = match open_for_portal(&path) {
                Ok(fd) => fd,
                Err(err) => return Err(Error::Unknown { description: err.to_string() }),
            };

            match proxy.trash_file((&fd).into()) {
                Ok(code) => match code {
                    1 => {}
                    num => return Err(Error::Portal { status_code: Some(num), source: None }),
                },
                Err(err) => return Err(Error::Portal { status_code: None, source: Some(err) }),
            }
        }

        Ok(())
    }
}

fn unsupported(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, msg.into())
}

fn map_open_error(path: &Path, err: Errno) -> io::Error {
    // O_NOFOLLOW on a symlink gives ELOOP on Linux and EMLINK on FreeBSD.
    match err {
        Errno::LOOP | Errno::MLINK => unsupported(format!("path `{path:?}` contains a symlink")),
        other => io::Error::from(other),
    }
}

/// open without following symlinks \
/// symlinks cannot be trashed, see https://github.com/flatpak/xdg-desktop-portal/issues/518
pub fn open_for_portal(path: &Path) -> io::Result<OwnedFd> {
    let mut names = Vec::new();
    for c in path.components() {
        match c {
            Component::RootDir => {}
            Component::Normal(n) => names.push(n),
            _ => unreachable!("path is not canonicalized somehow: {path:?}"),
        }
    }

    // error should be triggered if the path is `/`
    let (last, parents) = names.split_last().ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;

    // Traversing known names only needs search permission, not directory read permission.
    let dir_flags = OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;

    let mut dir = open("/", dir_flags, Mode::empty())?;
    for name in parents {
        dir = openat(&dir, *name, dir_flags, Mode::empty()).map_err(|err| map_open_error(path, err))?;
    }

    // only open a file or directory
    let before = statat(&dir, *last, AtFlags::SYMLINK_NOFOLLOW)?;
    open_leaf_for_portal(&dir, last, path, &before)
}

fn open_leaf_for_portal(dir: &OwnedFd, name: &OsStr, path: &Path, before: &Stat) -> io::Result<OwnedFd> {
    let flags = match FileType::from_raw_mode(before.st_mode) {
        FileType::RegularFile => OFlags::RDWR | OFlags::NOFOLLOW,
        // Reject replacement symlinks even if they point to the original directory.
        FileType::Directory => OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW,
        FileType::Symlink => return Err(unsupported(format!("path leaf {path:?}: trashing symlinks is unsupported"))),
        other => return Err(unsupported(format!("unsupported file type: {other:?}"))),
    };

    let fd = openat(dir, name, flags | OFlags::CLOEXEC, Mode::empty()).map_err(|err| map_open_error(path, err))?;

    // check that the thing being trashed hasn't been swapped
    let after = fstat(&fd)?;
    if (after.st_dev, after.st_ino) != (before.st_dev, before.st_ino) {
        return Err(io::Error::new(io::ErrorKind::Other, "file changed while opening"));
    }

    if FileType::from_raw_mode(after.st_mode) == FileType::Directory {
        // The portal rejects O_PATH descriptors carrying O_NOFOLLOW. Reopen "."
        // through the checked descriptor so we keep the same directory without
        // resolving the original leaf again or following a replacement symlink.
        return Ok(openat(&fd, ".", OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC, Mode::empty())?);
    }

    Ok(fd)
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use std::fs::{self, File};
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::path::Path;
    use std::process::Command;

    use crate::tests::{get_unique_name, init_logging};
    use crate::{delete, delete_all};

    use super::open_leaf_for_portal;
    use rustix::fs::{open, statat, AtFlags, Mode, OFlags};

    fn is_program_in_path(program: &str) -> bool {
        let Some(paths) = std::env::var_os("PATH") else { return false };
        std::env::split_paths(&paths).any(|dir| dir.join(program).is_file())
    }

    #[test]
    #[serial]
    fn trash_single_file() {
        init_logging();

        let name = get_unique_name();

        File::create_new(&name).unwrap();
        delete(&name).unwrap();

        assert!(!Path::new(&name).exists());
    }

    #[test]
    #[serial]
    fn trash_multiple_files() {
        init_logging();

        let names: Vec<_> = (0..3).map(|i| format!("{}-{}", get_unique_name(), i)).collect();
        for name in &names {
            File::create_new(name).unwrap();
        }

        delete_all(&names).unwrap();

        for name in &names {
            assert!(!Path::new(name).exists(), "{} should have been trashed", name);
        }
    }

    #[test]
    #[serial]
    fn trash_directory() {
        init_logging();

        let name = get_unique_name();

        fs::create_dir(&name).unwrap();
        File::create_new(format!("{}/inner-file", name)).unwrap();
        delete(&name).unwrap();

        assert!(!Path::new(&name).exists());
    }

    #[test]
    #[serial]
    fn trash_file_without_read_permission_on_ancestors() {
        init_logging();

        // Keep the fixture on the same filesystem as the other portal tests.
        let temp = tempfile::tempdir_in(".").unwrap();
        let ancestor = temp.path().join("searchable");
        let parent = ancestor.join("writable");
        fs::create_dir_all(&parent).unwrap();
        let target = parent.join("file");
        File::create_new(&target).unwrap();

        let _restore_permissions = defer::defer(|| {
            fs::set_permissions(&ancestor, fs::Permissions::from_mode(0o700)).unwrap();
            fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
        });
        fs::set_permissions(&ancestor, fs::Permissions::from_mode(0o111)).unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o311)).unwrap();
        assert_eq!(fs::read_dir(&ancestor).unwrap_err().kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(fs::read_dir(&parent).unwrap_err().kind(), std::io::ErrorKind::PermissionDenied);

        delete(&target).unwrap();

        assert!(!target.exists(), "search permission on ancestors should be sufficient for trashing");
    }

    #[test]
    fn open_rejects_directory_replaced_by_symlink_to_same_inode() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("selected");
        let renamed = temp.path().join("renamed");
        fs::create_dir(&target).unwrap();
        File::create_new(target.join("payload")).unwrap();

        let parent = open(temp.path(), OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC, Mode::empty()).unwrap();
        let name = target.file_name().unwrap();
        let before = statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW).unwrap();

        // Replace the leaf after statat with a symlink to the very same directory,
        // so the device/inode comparison alone cannot detect the replacement.
        fs::rename(&target, &renamed).unwrap();
        symlink(&renamed, &target).unwrap();

        let result = open_leaf_for_portal(&parent, name, &target, &before);
        assert!(result.is_err(), "opening a replacement symlink to the same directory must fail");
        assert!(target.is_symlink(), "the replacement symlink must remain in place");
        assert!(renamed.join("payload").is_file(), "the original directory must remain intact");
    }

    #[test]
    #[serial]
    fn trash_fails_when_file_not_writable() {
        init_logging();

        let name = get_unique_name();
        File::create_new(&name).unwrap();
        fs::set_permissions(&name, fs::Permissions::from_mode(0o444)).unwrap();

        let result = delete(&name);

        // Restore permissions before asserting, so a failed assertion doesn't
        // leave a read-only file behind for the next test run to trip over.
        fs::set_permissions(&name, fs::Permissions::from_mode(0o644)).unwrap();

        assert!(result.is_err(), "trashing a read-only-opened file should fail, not silently succeed");
        assert!(Path::new(&name).exists(), "the file should be left in place when trashing fails");

        fs::remove_file(&name).unwrap();
    }

    #[test]
    #[serial]
    fn trash_fails_when_symlink() {
        init_logging();

        let (name, name2) = (get_unique_name(), get_unique_name());
        File::create_new(&name).unwrap();
        symlink(&name, &name2).unwrap();

        let result = delete(&name2);

        assert!(result.is_err(), "trashing a symlink should not succeed");
        assert!(
            result.unwrap_err().to_string().contains("symlinks"),
            "trashing a symlink should return an unsupported error"
        );

        fs::remove_file(&name).unwrap();
        fs::remove_file(&name2).unwrap();
    }

    #[test]
    #[serial]
    fn matches_gio_trash_behavior() {
        init_logging();

        if !is_program_in_path("gio") {
            eprintln!("skipping matches_gio_trash_behavior: `gio` not found in PATH");
            return;
        }

        let via_gio = get_unique_name();
        let via_crate = format!("{}-crate", via_gio);

        File::create_new(&via_gio).unwrap();
        File::create_new(&via_crate).unwrap();

        let gio_status = Command::new("gio").args(["trash", &via_gio]).status().unwrap();
        assert!(gio_status.success(), "`gio trash` itself failed; not a crate issue");

        delete(&via_crate).unwrap();

        assert!(!Path::new(&via_gio).exists());
        assert!(!Path::new(&via_crate).exists());
    }
}
