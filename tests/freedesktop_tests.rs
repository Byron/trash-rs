// Freedesktop trash tests that run entirely inside privileged Docker containers.
//
// Every test case spins up a fresh Ubuntu 24.04 container with CAP_SYS_ADMIN
// (needed for `mount`) and copies the `trash-test-helper` binary in via the
// Docker API. All filesystem mutations happen inside the container, so the
// host is never touched.
//
// Prerequisites:
// - Docker daemon running and accessible to the current user.

#![cfg(target_os = "linux")]

use std::path::{Path, PathBuf};
use testcontainers::{core::ExecCommand, runners::AsyncRunner, ContainerAsync, GenericImage, ImageExt};

const IMAGE: &str = "ubuntu";
const TAG: &str = "24.04";

// ── helpers ──────────────────────────────────────────────────────────────────

/// Locate the compiled `trash-test-helper` binary using Cargo's integration-test
/// binary path environment variable.
fn find_trash_test_helper() -> PathBuf {
    let helper = PathBuf::from(env!("CARGO_BIN_EXE_trash-test-helper"));
    assert!(helper.exists(), "trash-test-helper not found at {helper:?}",);
    helper
}

/// Start a privileged container with the `trash-test-helper` binary copied in.
async fn start_container(helper: &Path) -> ContainerAsync<GenericImage> {
    let container = GenericImage::new(IMAGE, TAG)
        // Keep the container alive for the duration of the test.
        .with_cmd(["sleep", "infinity"])
        // CAP_SYS_ADMIN is required for `mount` inside the container.
        .with_privileged(true)
        .with_copy_to("/usr/local/bin/trash-test-helper", helper.to_path_buf())
        .start()
        .await
        .expect("failed to start container");

    // Ensure the copied binary is executable inside the container.
    exec_ok(&container, "chmod +x /usr/local/bin/trash-test-helper").await;
    container
}

/// Execute a shell command inside the container and return its exit code.
///
/// testcontainers 0.23 runs exec in detached mode, so `exit_code()` can
/// return `Ok(None)` until the process actually exits.  We poll until the
/// exit code appears (up to 5 s).
async fn exec_cmd(container: &ContainerAsync<GenericImage>, cmd: &str) -> i64 {
    let result = container
        .exec(ExecCommand::new(["sh", "-c", cmd]))
        .await
        .unwrap_or_else(|e| panic!("exec({cmd:?}) failed to launch: {e}"));

    for attempt in 0..50 {
        match result.exit_code().await {
            Ok(Some(code)) => return code,
            Ok(None) => {
                if attempt == 49 {
                    panic!("exec({cmd:?}) never exited after 5 s");
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            Err(e) => panic!("exit_code() for exec({cmd:?}) failed: {e}"),
        }
    }
    unreachable!()
}

/// Execute a setup command and panic if it fails.
async fn exec_ok(container: &ContainerAsync<GenericImage>, cmd: &str) {
    let code = exec_cmd(container, cmd).await;
    assert_eq!(code, 0, "setup command exited {code}: {cmd}");
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ComplexMount {
    A,
    B,
}

impl ComplexMount {
    const fn label(self) -> &'static str {
        match self {
            Self::A => "mount_a",
            Self::B => "mount_b",
        }
    }

    const fn direct_home(self) -> &'static str {
        match self {
            Self::A => "/foo/alice/john",
            Self::B => "/foo/bar/beth/john",
        }
    }

    const fn symlink_home(self) -> &'static str {
        match self {
            Self::A => "/foo/bar/baz/john",
            Self::B => "/foo/bridge/john",
        }
    }

    const fn trash_dir(self) -> &'static str {
        match self {
            Self::A => "/foo/.Trash-0",
            Self::B => "/foo/bar/.Trash-0",
        }
    }

    const fn other(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }

    fn home(self, access: AccessPath) -> &'static str {
        match access {
            AccessPath::Direct => self.direct_home(),
            AccessPath::ViaSymlink => self.symlink_home(),
        }
    }

    fn file_path(self, access: AccessPath, file_name: &str) -> String {
        format!("{}/{file_name}", self.home(access))
    }
}

#[derive(Clone, Copy, Debug)]
enum AccessPath {
    Direct,
    ViaSymlink,
}

impl AccessPath {
    const fn label(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::ViaSymlink => "via_symlink",
        }
    }
}

async fn setup_complex_mount_permutation_layout(container: &ContainerAsync<GenericImage>) {
    exec_ok(
        container,
        "mkdir -p /foo && \
         mount -t tmpfs tmpfs /foo && \
         mkdir -p /foo/bar && \
         mount -t tmpfs tmpfs /foo/bar && \
         mkdir -p /foo/alice/john && \
         mkdir -p /foo/bar/beth/john && \
         ln -s /foo/alice /foo/bar/baz && \
         ln -s /foo/bar/beth /foo/bridge",
    )
    .await;
}

async fn assert_complex_mount_permutation(
    container: &ContainerAsync<GenericImage>,
    file_mount: ComplexMount,
    file_access: AccessPath,
    home_mount: ComplexMount,
    home_access: AccessPath,
) {
    let case_name = format!(
        "file_{}_{}_home_{}_{}",
        file_mount.label(),
        file_access.label(),
        home_mount.label(),
        home_access.label(),
    );
    let file_name = format!("doe-{case_name}");
    let file_path = file_mount.file_path(file_access, &file_name);
    let home_data_dir = home_mount.home(home_access);

    exec_ok(container, &format!("touch {file_path}")).await;

    let delete_cmd = format!("XDG_DATA_HOME={home_data_dir} /usr/local/bin/trash-test-helper delete {file_path}");
    let code = exec_cmd(container, &delete_cmd).await;
    assert_eq!(code, 0, "{case_name}: delete should succeed");

    let in_home_trash = exec_cmd(container, &format!("test -f {home_data_dir}/Trash/files/{file_name}")).await;
    let in_file_mount_trash =
        exec_cmd(container, &format!("test -e {}/files/{file_name}", file_mount.trash_dir())).await;
    let in_other_mount_trash =
        exec_cmd(container, &format!("test -e {}/files/{file_name}", file_mount.other().trash_dir())).await;

    if file_mount == home_mount {
        assert_eq!(in_home_trash, 0, "{case_name}: file must be in the home trash");
        assert_ne!(in_file_mount_trash, 0, "{case_name}: file must not fall back to the file mount trash");
    } else {
        assert_ne!(in_home_trash, 0, "{case_name}: file must not land in the home trash");
        assert_eq!(in_file_mount_trash, 0, "{case_name}: file must land in the file mount trash");
    }

    assert_ne!(in_other_mount_trash, 0, "{case_name}: file must not land in the unrelated mount trash");
}

// ── test cases ────────────────────────────────────────────────────────────────

/// The home trash directory (`$HOME/.local/share/Trash`) is a regular directory.
/// Deleting a file should succeed and place it under `Trash/files/`.
#[tokio::test]
async fn trash_is_dir() {
    let helper = find_trash_test_helper();
    let c = start_container(&helper).await;

    exec_ok(&c, "mkdir -p /home/u/.local/share/Trash && touch /target-file").await;

    let code = exec_cmd(&c, "HOME=/home/u /usr/local/bin/trash-test-helper delete /target-file").await;
    assert_eq!(code, 0, "delete to a directory trash should succeed");

    let verify = exec_cmd(&c, "test -f /home/u/.local/share/Trash/files/target-file").await;
    assert_eq!(verify, 0, "file should appear in Trash/files/");
}

/// The home trash path is a regular *file* (not a directory).
/// The trash operation should fail because it cannot create subdirectories inside it.
#[tokio::test]
async fn trash_is_file() {
    let helper = find_trash_test_helper();
    let c = start_container(&helper).await;

    exec_ok(&c, "mkdir -p /home/u/.local/share && touch /home/u/.local/share/Trash && touch /target-file").await;

    let code = exec_cmd(&c, "HOME=/home/u /usr/local/bin/trash-test-helper delete /target-file").await;
    assert_ne!(code, 0, "delete when Trash is a file should fail");

    // The source file must still be present.
    let still_there = exec_cmd(&c, "test -f /target-file").await;
    assert_eq!(still_there, 0, "source file must not have been removed on failure");
}

/// The home trash path is a symbolic link that points to a *directory*.
/// This is valid – the library follows the symlink and uses the target directory.
#[tokio::test]
async fn trash_is_symlink_to_dir() {
    let helper = find_trash_test_helper();
    let c = start_container(&helper).await;

    exec_ok(
        &c,
        "mkdir /actual-trash && \
         mkdir -p /home/u/.local/share && \
         ln -s /actual-trash /home/u/.local/share/Trash && \
         touch /target-file",
    )
    .await;

    let code = exec_cmd(&c, "HOME=/home/u /usr/local/bin/trash-test-helper delete /target-file").await;
    assert_eq!(code, 0, "delete via a symlink-to-dir trash should succeed");

    // The file ends up in the *real* directory the symlink points to.
    let verify = exec_cmd(&c, "test -f /actual-trash/files/target-file").await;
    assert_eq!(verify, 0, "file should appear in the real trash directory");
}

/// The home trash path is a symbolic link that points to a *regular file*.
/// This is invalid; the trash operation should fail.
#[tokio::test]
async fn trash_is_symlink_to_file() {
    let helper = find_trash_test_helper();
    let c = start_container(&helper).await;

    exec_ok(
        &c,
        "touch /actual-file && \
         mkdir -p /home/u/.local/share && \
         ln -s /actual-file /home/u/.local/share/Trash && \
         touch /target-file",
    )
    .await;

    let code = exec_cmd(&c, "HOME=/home/u /usr/local/bin/trash-test-helper delete /target-file").await;
    assert_ne!(code, 0, "delete when Trash symlink points to a file should fail");

    let still_there = exec_cmd(&c, "test -f /target-file").await;
    assert_eq!(still_there, 0, "source file must not have been removed on failure");
}

/// The home trash path is a *broken* symbolic link (the target does not exist).
/// The trash operation should fail.
#[tokio::test]
async fn trash_is_symlink_to_nonexistent() {
    let helper = find_trash_test_helper();
    let c = start_container(&helper).await;

    exec_ok(
        &c,
        "mkdir -p /home/u/.local/share && \
         ln -s /does-not-exist /home/u/.local/share/Trash && \
         touch /target-file",
    )
    .await;

    let code = exec_cmd(&c, "HOME=/home/u /usr/local/bin/trash-test-helper delete /target-file").await;
    assert_ne!(code, 0, "delete when Trash is a broken symlink should fail");

    let still_there = exec_cmd(&c, "test -f /target-file").await;
    assert_eq!(still_there, 0, "source file must not have been removed on failure");
}

/// The home trash directory is itself a *mount point* (a separate tmpfs).
///
/// Because the source file (`/target-file`) lives on the root filesystem and the
/// home trash lives on its own mount, the library correctly identifies that they
/// are on different filesystems.  It therefore creates `/.Trash-0/` (the per-UID
/// trash on the root mount) instead of using the home trash.
#[tokio::test]
async fn trash_is_mount() {
    let helper = find_trash_test_helper();
    let c = start_container(&helper).await;

    exec_ok(
        &c,
        "mkdir -p /home/u/.local/share/Trash && \
         mount -t tmpfs tmpfs /home/u/.local/share/Trash && \
         touch /target-file",
    )
    .await;

    let code = exec_cmd(&c, "HOME=/home/u /usr/local/bin/trash-test-helper delete /target-file").await;
    assert_eq!(code, 0, "delete should succeed even when Trash is on its own mount");

    // The file is on the root FS; the library places it in the root FS's per-UID trash.
    let verify = exec_cmd(&c, "test -f /.Trash-0/files/target-file").await;
    assert_eq!(verify, 0, "file should be in /.Trash-0/ (the root FS trash)");

    // The home trash mount must remain empty.
    let home_trash_empty = exec_cmd(&c, "test -z \"$(ls /home/u/.local/share/Trash/files/ 2>/dev/null)\"").await;
    assert_eq!(home_trash_empty, 0, "home Trash mount should be empty");
}

/// Complex mount/symlink scenario:
///
/// ```text
///   /foo          — tmpfs mount A
///   /foo/bar      — tmpfs mount B (separate filesystem inside A)
///   /foo/bar/baz  — symlink → /foo/alice   (on mount B)
///   /foo/alice/   — directory on mount A
/// ```
///
/// The file to delete is `/foo/bar/baz/john/doe`.
/// After symlink resolution its canonical path is `/foo/alice/john/doe`,
/// which lives on mount A (`/foo`), **not** on mount B (`/foo/bar`).
///
/// The library must resolve symlinks before looking up the mount point, so the
/// trash should end up in `/foo/.Trash-0/` and *not* in `/foo/bar/.Trash-0/`.
#[tokio::test]
async fn trash_complex_mounts_with_symlink() {
    let helper = find_trash_test_helper();
    let c = start_container(&helper).await;

    exec_ok(
        &c,
        // Build the described layout step by step.
        "mkdir -p /foo && \
         mount -t tmpfs tmpfs /foo && \
         mkdir -p /foo/bar && \
         mount -t tmpfs tmpfs /foo/bar && \
         mkdir -p /foo/alice/john && \
         ln -s /foo/alice /foo/bar/baz && \
         touch /foo/bar/baz/john/doe",
        // ↑ creates /foo/alice/john/doe through the symlink
    )
    .await;

    // Put the home directory on the root FS so it belongs to a different mount.
    exec_ok(&c, "mkdir -p /home/u/.local/share/Trash").await;

    let code = exec_cmd(&c, "HOME=/home/u /usr/local/bin/trash-test-helper delete /foo/bar/baz/john/doe").await;
    assert_eq!(code, 0, "delete should succeed");

    // Canonical path is /foo/alice/john/doe → mount point is /foo.
    // The trash must be /foo/.Trash-0/files/doe.
    let in_foo_trash = exec_cmd(&c, "test -f /foo/.Trash-0/files/doe").await;
    assert_eq!(in_foo_trash, 0, "file must be trashed under /foo/.Trash-0/ (mount A)");

    // Must NOT appear under mount B's trash.
    let in_bar_trash = exec_cmd(&c, "test -e /foo/bar/.Trash-0/files/doe").await;
    assert_ne!(in_bar_trash, 0, "file must NOT be in /foo/bar/.Trash-0/ (wrong mount)");

    // Must NOT appear in the home trash.
    let in_home_trash = exec_cmd(&c, "test -e /home/u/.local/share/Trash/files/doe").await;
    assert_ne!(in_home_trash, 0, "file must NOT be in home Trash (different mount)");
}

/// Variant of `trash_complex_mounts_with_symlink` where the user's home trash
/// is itself reachable only through the symlink (`XDG_DATA_HOME=/foo/bar/baz/john`).
///
/// Layout (same mounts and symlink as the previous test):
///
/// ```text
///   /foo              — tmpfs mount A
///   /foo/bar          — tmpfs mount B
///   /foo/bar/baz      — symlink → /foo/alice
///   /foo/alice/john/  — directory on mount A
/// ```
///
/// `XDG_DATA_HOME=/foo/bar/baz/john` → home trash = `/foo/bar/baz/john/Trash`
///
/// After symlink resolution that is `/foo/alice/john/Trash`, which lives on
/// mount A — the **same** mount as the deleted file (`/foo/alice/john/doe`).
///
/// Therefore the library should use the home trash directly instead of
/// creating a per-mount `.Trash-0` directory.
#[tokio::test]
async fn trash_complex_mounts_home_trash_via_symlink() {
    let helper = find_trash_test_helper();
    let c = start_container(&helper).await;

    exec_ok(
        &c,
        "mkdir -p /foo && \
         mount -t tmpfs tmpfs /foo && \
         mkdir -p /foo/bar && \
         mount -t tmpfs tmpfs /foo/bar && \
         mkdir -p /foo/alice/john && \
         ln -s /foo/alice /foo/bar/baz && \
         touch /foo/bar/baz/john/doe",
    )
    .await;

    let code =
        exec_cmd(&c, "XDG_DATA_HOME=/foo/bar/baz/john /usr/local/bin/trash-test-helper delete /foo/bar/baz/john/doe")
            .await;
    assert_eq!(code, 0, "delete should succeed");

    // The home trash canonicalizes to /foo/alice/john/Trash (mount A), which
    // is the same mount as the file.  The file must land there.
    let in_home_trash = exec_cmd(&c, "test -f /foo/bar/baz/john/Trash/files/doe").await;
    assert_eq!(in_home_trash, 0, "file must be in the home trash (/foo/alice/john/Trash)");

    // Must NOT fall back to the per-mount trash on /foo.
    let in_foo_trash = exec_cmd(&c, "test -e /foo/.Trash-0/files/doe").await;
    assert_ne!(in_foo_trash, 0, "file must NOT be in /foo/.Trash-0/");

    // Must NOT land in /foo/bar's trash (wrong mount).
    let in_bar_trash = exec_cmd(&c, "test -e /foo/bar/.Trash-0/files/doe").await;
    assert_ne!(in_bar_trash, 0, "file must NOT be in /foo/bar/.Trash-0/");
}

#[tokio::test]
async fn trash_complex_mounts_home_trash_permutations() {
    let helper = find_trash_test_helper();
    let c = start_container(&helper).await;
    setup_complex_mount_permutation_layout(&c).await;

    for mount in [ComplexMount::A, ComplexMount::B] {
        for file_access in [AccessPath::Direct, AccessPath::ViaSymlink] {
            for home_access in [AccessPath::Direct, AccessPath::ViaSymlink] {
                assert_complex_mount_permutation(&c, mount, file_access, mount, home_access).await;
            }
        }
    }
}

#[tokio::test]
async fn trash_complex_mounts_per_mount_trash_permutations() {
    let helper = find_trash_test_helper();
    let c = start_container(&helper).await;
    setup_complex_mount_permutation_layout(&c).await;

    for file_mount in [ComplexMount::A, ComplexMount::B] {
        let home_mount = file_mount.other();
        for file_access in [AccessPath::Direct, AccessPath::ViaSymlink] {
            for home_access in [AccessPath::Direct, AccessPath::ViaSymlink] {
                assert_complex_mount_permutation(&c, file_mount, file_access, home_mount, home_access).await;
            }
        }
    }
}
