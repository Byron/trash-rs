
use std::path::Path;

use crate::Error;

pub fn is_implemented() -> bool {
    false
}

pub fn remove_all<I, T>(paths: I) -> Result<(), Error>
where
    I: IntoIterator<Item = T>,
    T: AsRef<Path>,
{
    unimplemented!();
}

pub fn remove<T: AsRef<Path>>(path: T) -> Result<(), Error> {
    unimplemented!();
}
