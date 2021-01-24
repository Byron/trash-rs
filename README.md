
[![Crates.io](https://img.shields.io/crates/v/trash?color=mediumvioletred)](https://crates.io/crates/trash)
[![Docs.rs](https://docs.rs/trash/badge.svg)](https://docs.rs/trash)



## About

The `trash` is a Rust library for moving files and folders to the operating system's Recycle Bin or Trash or Rubbish Bin or what have you :D

The library supports Windows, macOS, Linux, and BSD.

## Usage

```toml
# In Cargo.toml
[dependencies]
trash = "1.2"
```

```rust
// In main.rs
use std::fs::File;
use trash;

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
```

## Note on Version 2

Version 2 would add `list`, `purge_all`, and `restore_all` that would allow for listing, permanently removing or restoring trashed items.
Development for Version 2 is currently suspended as I couldn't manage to get these features to work on Mac. Contribution would be very welcome.
An imperfect alternative would be to release those features for Linux and Windows both of which at this point have a more or less complete implementation of these features on the `v2-dev` branch. The windows implementation depends on a custom fork of winapi, because the PR adding the required features cannot be merged before winapi 0.4. I would like to merge the branch adding `list`, `purge_all`, and `restore_all` only after the winapi PR is is merged and published.
