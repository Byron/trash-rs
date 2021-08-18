use std::fs::File;

fn main() {
    // Let's create and remove a single file
    File::create("remove-me").unwrap();
    trash::delete("remove-me").unwrap();
    assert!(File::open("remove-me").is_err());

    // Now let's remove multiple files at once
    let the_others = ["remove-me-too", "dont-forget-about-me-either"];
    for name in the_others.iter() {
        File::create(name).unwrap();
    }
    trash::delete_all(&the_others).unwrap();
    for name in the_others.iter() {
        assert!(File::open(name).is_err());
    }
}
