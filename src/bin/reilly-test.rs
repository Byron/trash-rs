use std::fs::File;

use trash::delete;

fn main() {
    println!("Hello World!");
    let path = "file_to_delete.txt".to_string();
    println!("Deleting {path}...");
    File::create(&path).unwrap();
    delete(&path).unwrap();
    println!("Done!");
}
