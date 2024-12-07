mod utils {

    use std::sync::atomic::{AtomicI32, Ordering};

    use once_cell::sync::Lazy;

    // WARNING Expecting that `cargo test` won't be invoked on the same computer more than once within
    // a single millisecond
    static INSTANCE_ID: Lazy<i64> = Lazy::new(|| chrono::Local::now().timestamp_millis());
    static ID_OFFSET: AtomicI32 = AtomicI32::new(0);

    pub fn get_unique_name() -> String {
        let id = ID_OFFSET.fetch_add(1, Ordering::SeqCst);
        format!("trash-test-{}-{}", *INSTANCE_ID, id)
    }

    pub fn init_logging() {
        let _ = env_logger::builder().is_test(true).try_init();
    }
}

pub use utils::{get_unique_name, init_logging};

#[cfg(any(
    target_os = "windows",
    all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android"))
))]
mod os_limited {
    use super::{get_unique_name, init_logging};
    use serial_test::serial;
    use std::collections::{hash_map::Entry, HashMap};
    use std::ffi::{OsStr, OsString};
    use std::fs::File;

    #[cfg(all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android")))]
    use std::os::unix::ffi::OsStringExt;

    use crate as trash;
}
