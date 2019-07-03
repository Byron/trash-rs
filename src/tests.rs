use crate::remove;
use std::fs::File;

#[test]
fn create_remove() {
    let path = "test_file_to_remove";
    File::create(path).unwrap();

    remove(path).unwrap();
    assert!(File::open(path).is_err());
}
