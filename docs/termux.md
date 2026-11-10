# Building Dupsonic without trash support in Termux

[Issue #162](https://github.com/Byron/trash-rs/issues/162) reports unresolved
`platform` and `delete_all_canonicalized` errors when building
[Dupsonic](https://github.com/zas/dupsonic) in Termux with `trash` 5.3.0.

Termux uses Rust's Android target (`target_os = "android"`), which `trash` does
not support. Both the backend modules and their dependencies exclude Android.
Dupsonic already enables the default `freedesktop` feature through `trash = "5"`;
adding that feature explicitly produces the same errors. This is distinct from
the Linux error in the issue comments, where a consumer disables default
features without selecting a backend.

`--locked` only reuses an existing Cargo.lock; it does not add platform support.
Dupsonic does not track Cargo.lock at the revision below, so a fresh source copy
has no lockfile to reuse.

## Consumer workaround

The [optional-trash patch](dupsonic-optional-trash.patch) applies to Dupsonic
0.2.6 at commit
[`5fa918a`](https://github.com/zas/dupsonic/tree/5fa918a58a55662b05176f995be77c553802c801).
It makes the `trash` dependency optional behind a default-enabled Dupsonic
feature named `trash`. Building with `--no-default-features` then removes that
dependency entirely.

The patch also guards all calls to `trash` in `src/web.rs`. Without the feature,
web delete and restore requests return HTTP 501 with an explanatory message.
Files are left intact; duplicate scanning and exclusion remain available.
The web UI still displays the action buttons and reports the server's error
when they are used. Default builds on supported platforms retain trash support.

Save the linked patch as `dupsonic-optional-trash.patch`. From the Dupsonic source
directory, apply it and build using Termux's native Rust and C toolchains:

```sh
git apply --check /path/to/dupsonic-optional-trash.patch
git apply /path/to/dupsonic-optional-trash.patch
cargo build --release --no-default-features
```

To install from the patched local source instead:

```sh
cargo install --path . --no-default-features
```

These commands require the patch first: the original Dupsonic dependency is
unconditional, so `--no-default-features` alone does not remove `trash`.
Keep using this flag when rebuilding in Termux. Do not add `--features trash`
there, as that reintroduces the unsupported dependency.

## Reproduction and validation

The original consumer dependency graph reproduces the reported compiler errors
on a Linux host with the Android Rust standard library installed:

```sh
rustup target add aarch64-linux-android
cargo check --target aarch64-linux-android -p trash
```

The same check fails after changing the original dependency to
`trash = { version = "5.3.0", features = ["freedesktop"] }`.

After applying the patch:

```sh
cargo tree --target aarch64-linux-android --no-default-features
cargo test --no-default-features
cargo test
```

The Android dependency graph no longer contains `trash`. On Linux, the build
without default features passes 53 unit tests and 18 integration tests; the
default build passes 52 unit tests and 18 integration tests. The included
regression test checks that delete and restore are rejected without trash
support, the file's bytes are preserved, and exclusion still succeeds. Before
the patch, that test failed because the file was trashed.

A full Android cross-check with `cargo check --target aarch64-linux-android
--no-default-features` reached the consumer's bundled SQLite dependency and
stopped because the host lacked `aarch64-linux-android-clang`. This validates
removal of the reported `trash` blocker, but a complete build and runtime test
on a Termux device remain unverified. The patch does not implement Android
trash support.
