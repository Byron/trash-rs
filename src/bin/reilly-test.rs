use std::fs::File;
use anyhow::Result;

use trash::{delete, os_limited::list};

fn main() -> Result<()> {
    // println!("Hello World!");
    // let path = "file_to_delete.txt".to_string();
    // println!("Deleting {path}...");
    // File::create(&path).unwrap();
    // delete(&path).unwrap();

    let l = list()?;

    // println!("Done!");
    Ok(())
}
