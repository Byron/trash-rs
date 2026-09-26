use std::fs::OpenOptions;
use std::os::fd::AsFd;
use std::path::PathBuf;

use once_cell::sync::OnceCell;
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
            let mut file = OpenOptions::new();

            file.read(true);

            if path.is_file() {
                file.write(true);
            }

            let file = match file.open(path) {
                Ok(file) => file,
                Err(err) => return Err(Error::Unknown { description: err.to_string() }),
            };

            match proxy.trash_file(file.as_fd().into()) {
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

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use std::fs::{self, File};
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::process::Command;

    use crate::tests::{get_unique_name, init_logging};
    use crate::{delete, delete_all};

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
