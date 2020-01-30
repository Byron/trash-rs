
[![Crates.io](https://img.shields.io/crates/v/trash.svg)](https://crates.io/crates/trash)
[![Docs.rs](https://docs.rs/trash/badge.svg)](https://docs.rs/trash)

## About

Trash is a Rust library that provides functionality to move files and folders to the operating system's Recycle Bin (or Trash or Rubbish Bin or what have you).

The library supports Windows, Mac and Linux.

Version 2 would add `list`, `purge_all`, and `restore_all` that would allow for listing, permanently removing or restoring trashed items.
Development for Version 2 is currently suspended as I couldn't manage to get these features to work on Mac.
An imperfect alternative would be to release those features for Linux and Windows both of which at this point have a more or less complete implementation of these features on the `v2-dev` branch but currently I have other priorities. If you'd be interested having access to the above mentionned on Windows and Linux feel free to open an issue.

## Usage

```rust
extern crate trash;
use std::fs::File;

fn main() {
    // Let's create and remove a single file
    File::create("remove-me").unwrap();
    trash::remove("remove-me").unwrap();
    assert!(File::open("remove-me").is_err());

    // Now let's remove multiple files at once
    let the_others = ["remove-me-too", "dont-forget-about-me-either"];
    for name in the_others.iter() {
        File::create(name).unwrap();
    }
    trash::remove_all(&the_others).unwrap();
    for name in the_others.iter() {
        assert!(File::open(name).is_err());
    }
}
```
