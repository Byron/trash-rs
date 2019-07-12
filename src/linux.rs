
use std::path::Path;

use crate::Error;

pub fn is_implemented() -> bool {
    true
}

pub fn platform_remove<T: AsRef<Path>>(path: T) -> Result<(), Error> {
    unimplemented!();
}
