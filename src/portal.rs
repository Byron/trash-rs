use std::fs::OpenOptions;
use std::path::PathBuf;
use std::os::fd::AsFd;

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
		Self {
			conn: OnceCell::new(),
			proxy: OnceCell::new(),
		}
	}

	fn connection(&self) -> Result<&zbus::blocking::Connection, Error> {
		self.conn.get_or_try_init(|| {
			zbus::blocking::Connection::session().map_err(|err| Error::Portal {
				status_code: None,
				source: Some(err),
			})
		})
	}

	fn proxy(&self) -> Result<&TrashProxy<'_>, Error> {
		let conn = self.connection()?;

		self.proxy.get_or_try_init(|| {
			TrashProxy :: new(conn).map_err(|err| Error::Portal {
				status_code: None,
				source: Some(err),
			})
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
				Err(err) => {
					return Err(Error::Unknown { description: err.to_string() })
				}
			};

			match proxy.trash_file(file.as_fd().into()) {
				Ok(code) => {
					match code {
						1 => {},
						num => return Err(Error::Portal {
							status_code: Some(num),
							source: None
						})
					}
				},
				Err(err) => return Err(Error::Portal {
					status_code: None,
					source: Some(err.into())
				})
			}
		}

		Ok(())
	}
}
