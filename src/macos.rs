
use std::path::Path;

use crate::Error;

pub fn is_implemented() -> bool {
    false
}

pub fn remove<T: AsRef<Path>>(path: T) -> Result<(), Error> {
    unimplemented!();
}
