# trash

[![Crates.io](https://img.shields.io/crates/v/trash.svg)](https://crates.io/crates/trash)
[![Docs.rs](https://docs.rs/trash/badge.svg)](https://docs.rs/trash)

Trash is a Rust library for moving files and folders to the OS's Trash or Recycle Bin.

Supported platforms:

- Linux/*BSD: [The FreeDesktop.org Trash specification](https://specifications.freedesktop.org/trash-spec/trashspec-1.0.html)
- macOS
- Windows

## Usage

Add the following to your Cargo.toml:

```toml
[dependencies]
trash = "1.1.1"
```

## Example

```rust
use std::fs::File;

fn main() {
    // Let's create and remove a single file
    let file = "remove-me";
    File::create(file).unwrap();
    trash::remove(file).unwrap();
    assert!(File::open(file).is_err());
    
    // Now let's remove multiple files at once
    let multiple_files = ["remove-me-too", "dont-forget-about-me-either"];
    for name in multiple_files.iter() {
        File::create(name).unwrap();
    }
    trash::remove_all(&multiple_files).unwrap();
    for name in multiple_files.iter() {
        assert!(File::open(name).is_err());
    }
}
```

## Note on Version 2

Version 2 would add `list`, `purge_all`, and `restore_all` that would allow for listing, permanently removing or restoring trashed items.
Development for Version 2 is currently suspended as I couldn't manage to get these features to work on Mac. Contribution would be very welcome.
An imperfect alternative would be to release those features for Linux and Windows both of which at this point have a more or less complete implementation of these features on the `v2-dev` branch. The windows implementation depends on a custom fork of winapi, because the PR adding the required features cannot be merged before winapi 0.4. I would like to merge the branch adding `list`, `purge_all`, and `restore_all` only after the winapi PR is is merged and published.

## Apps using trash

- [emulsion](https://github.com/ArturKovacs/emulsion)
- [nushell](https://github.com/nushell/nushell)

## Related projects

- sindresorhus
  - [empty-trash](https://github.com/sindresorhus/empty-trash)
  - [trash](https://github.com/sindresorhus/trash)
- [xdg](https://github.com/rkoesters/xdg)
