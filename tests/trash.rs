use trash::delete;

mod util {
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
pub use util::{get_unique_name, init_logging};

#[cfg(unix)]
mod unix {
    use log::trace;
    use std::{
        fs::{create_dir, remove_dir_all, remove_file, File},
        os::unix::fs::symlink,
        path::Path,
    };

    use super::{get_unique_name, init_logging};
    use crate::delete;
    // use crate::init_logging;

    #[test]
    fn test_delete_symlink() {
        init_logging();
        trace!("Started test_delete_symlink");
        let target_path = get_unique_name();
        File::create_new(&target_path).unwrap();

        let link_path = "test_link_to_delete";
        symlink(&target_path, link_path).unwrap();

        delete(link_path).unwrap();
        assert!(File::open(link_path).is_err());
        assert!(File::open(&target_path).is_ok());
        // Cleanup
        remove_file(&target_path).unwrap();
        trace!("Finished test_delete_symlink");
    }

    #[test]
    fn test_delete_symlink_in_folder() {
        init_logging();
        trace!("Started test_delete_symlink_in_folder");
        let target_path = "test_link_target_for_delete_from_folder";
        File::create_new(target_path).unwrap();

        let folder = Path::new("test_parent_folder_for_link_to_delete");
        create_dir(folder).unwrap();
        let link_path = folder.join("test_link_to_delete_from_folder");
        symlink(target_path, &link_path).unwrap();

        delete(&link_path).unwrap();
        assert!(File::open(link_path).is_err());
        assert!(File::open(target_path).is_ok());
        // Cleanup
        remove_file(target_path).unwrap();
        remove_dir_all(folder).unwrap();
        trace!("Finished test_delete_symlink_in_folder");
    }
}
