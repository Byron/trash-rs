use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Error;

static DEFAULT_TRASH: &str = "gio";

pub fn is_implemented() -> bool {
	true
}

/// This is based on the electron library's implementation.
/// See: https://github.com/electron/electron/blob/34c4c8d5088fa183f56baea28809de6f2a427e02/shell/common/platform_util_linux.cc#L96
pub fn remove_all_canonicalized(full_paths: Vec<PathBuf>) -> Result<(), Error> {
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
	let result = command.output().map_err(|e| Error::Remove { code: e.raw_os_error() })?;

	if !result.status.success() {
		return Err(Error::Remove { code: result.status.code() });
	}

	Ok(())
}

pub fn remove_all<I, T>(paths: I) -> Result<(), Error>
where
	I: IntoIterator<Item = T>,
	T: AsRef<Path>,
{
	let paths = paths.into_iter();
	let full_paths = paths
		.map(|x| x.as_ref().canonicalize())
		.collect::<Result<Vec<_>, _>>()
		.map_err(|e| Error::CanonicalizePath { code: e.raw_os_error() })?;

	remove_all_canonicalized(full_paths)
}

pub fn remove<T: AsRef<Path>>(path: T) -> Result<(), Error> {
	remove_all(&[path])
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
		for value in xdg_current_desktop.split(':') {
			let value = value.trim();
			if value.is_empty() {
				continue;
			}
			match value {
				"Unity" => {
					// gnome-fallback sessions set XDG_CURRENT_DESKTOP to Unity
					// DESKTOP_SESSION can be gnome-fallback or gnome-fallback-compiz
					if let Ok(desktop_session) = env::var("DESKTOP_SESSION") {
						if desktop_session.contains("gnome-fallback") {
							return DesktopEnvironment::Gnome;
						}
					}
					return DesktopEnvironment::Unity;
				}
				"GNOME" => {
					return DesktopEnvironment::Gnome;
				}
				"X-Cinnamon" => {
					return DesktopEnvironment::Cinnamon;
				}
				"KDE" => {
					if let Ok(kde_session) = env::var(KDE_SESSION_ENV_VAR) {
						if kde_session == "5" {
							return DesktopEnvironment::Kde5;
						}
					}
					return DesktopEnvironment::Kde4;
				}
				"Pantheon" => {
					return DesktopEnvironment::Pantheon;
				}
				"XFCE" => {
					return DesktopEnvironment::Xfce;
				}
				_ => {}
			}
		}
	}

	// DESKTOP_SESSION was what everyone  used in 2010.
	if let Ok(desktop_session) = env::var("DESKTOP_SESSION") {
		match desktop_session.as_str() {
			"gnome" | "mate" => {
				return DesktopEnvironment::Gnome;
			}
			"kde4" | "kde-plasma" => {
				return DesktopEnvironment::Kde4;
			}
			"kde" => {
				// This may mean KDE4 on newer systems, so we have to check.
				if env_has_var(KDE_SESSION_ENV_VAR) {
					return DesktopEnvironment::Kde4;
				}
				return DesktopEnvironment::Kde3;
			}
			"xubuntu" => {
				return DesktopEnvironment::Xfce;
			}
			_ => {}
		}
		if desktop_session.contains("xfce") {
			return DesktopEnvironment::Xfce;
		}
	}

	// Fall back on some older environment variables.
	// Useful particularly in the DESKTOP_SESSION=default case.
	if env_has_var("GNOME_DESKTOP_SESSION_ID") {
		return DesktopEnvironment::Gnome;
	} else if env_has_var("KDE_FULL_SESSION") {
		if env_has_var(KDE_SESSION_ENV_VAR) {
			return DesktopEnvironment::Kde4;
		}
		return DesktopEnvironment::Kde3;
	}

	DesktopEnvironment::Other
}
