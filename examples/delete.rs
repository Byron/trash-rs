use std::env;
use std::path::PathBuf;
use trash::{delete_all, Error};

#[cfg(any(
    target_os = "windows",
    all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android"))
))]
fn main() -> Result<(), Error> {
    let args: Vec<PathBuf> = env::args().skip(1).map(String::into).collect();
    delete_all(args)?;

    Ok(())
}
