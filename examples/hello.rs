use std::fs::{create_dir, File};
use std::path::PathBuf;

fn main() {
    // Let's create and remove a single file
    File::create_new("remove-me").unwrap();
    trash::delete("remove-me").unwrap();
    assert!(File::open("remove-me").is_err());

    // Now let's remove multiple files at once
    let the_others = ["remove-me-too", "dont-forget-about-me-either"];
    for name in the_others.iter() {
        File::create_new(name).unwrap();
    }
    trash::delete_all(the_others).unwrap();
    for name in the_others.iter() {
        assert!(File::open(name).is_err());
    }

    // Now let's delete a directory
    create_dir("remove-me").unwrap();
    trash::delete("remove-me").unwrap();
    assert!(!PathBuf::from("remove-me").try_exists().unwrap());
}
