// Separate file to force running tests on the main thread, a must for macOS OSAkit APIs, which can otherwise fail after a 2-min stall
// Uses a custom libtest_mimic test harness for that since the default Cargo test doesn't support it.
// ADD "test_main_thread_" prefix to test names so that the main cargo test run filter them out with `--skip "test_main_thread_"`
fn main() {
    #[cfg(target_os = "macos")]
    test_main_thread_mac::main_mac()
}

#[cfg(target_os = "macos")]
#[path = "../src/tests.rs"]
mod trash_tests;

#[cfg(target_os = "macos")]
mod test_main_thread_mac {
    use crate::trash_tests::{get_unique_name, init_logging};
    use serial_test::serial;
    use std::{fs::File, path::PathBuf};
    use trash::{
        macos::{DeleteMethod, ScriptMethod, TrashContextExtMacos},
        TrashContext,
    }; // not pub, so import directly

    pub(crate) fn main_mac() {
        use libtest_mimic::{Arguments, Trial};
        let args = Arguments::from_args(); // Parse command line arguments
        let tests = vec![
            // Create a list of tests and/or benchmarks
            Trial::test("test_main_thread_delete_with_finder_osakit_with_info", || {
                Ok(test_main_thread_delete_with_finder_osakit_with_info())
            }),
        ];
        libtest_mimic::run(&args, tests).exit(); // Run all tests and exit the application appropriatly
    }

    use std::thread;
    #[serial]
    pub fn test_main_thread_delete_with_finder_osakit_with_info() {
        // OSAkit must be run on the main thread
        if let Some("main") = thread::current().name() {
        } else {
            eprintln!("This test is NOT thread-safe, so must be run on the main thread, and is not, thus can fail…");
        };

        init_logging();
        let mut trash_ctx = TrashContext::default();
        trash_ctx.set_delete_method(DeleteMethod::Finder);
        trash_ctx.set_script_method(ScriptMethod::Osakit);

        let mut path1 = PathBuf::from(get_unique_name());
        let mut path2 = PathBuf::from(get_unique_name());
        path1.set_extension(r#"a"b,"#);
        path2.set_extension(r#"x80=%80 slash=\ pc=% quote=" comma=,"#);
        for _i in 1..10 {
            // run a few times since previously threading issues didn't always appear on 1st run
            File::create_new(&path1).unwrap();
            File::create_new(&path2).unwrap();
            let trashed_items = trash_ctx.delete_all_with_info(&[path1.clone(), path2.clone()]).unwrap().unwrap(); //Ok + Some trashed paths
            assert!(File::open(&path1).is_err()); // original files deleted
            assert!(File::open(&path2).is_err());
            for item in trashed_items {
                let trashed_path = item.id;
                assert!(!File::open(&trashed_path).is_err()); // returned trash items exist
                std::fs::remove_file(&trashed_path).unwrap(); // clean   up
                assert!(File::open(&trashed_path).is_err()); // cleaned up trash items
            }
        }

        // test a single file (in case returned paths aren't an array anymore)
        let mut path3 = PathBuf::from(get_unique_name());
        path3.set_extension(r#"a"b,"#);
        File::create_new(&path3).unwrap();
        let item = trash_ctx.delete_with_info(&path3).unwrap().unwrap(); //Ok + Some trashed paths
        assert!(File::open(&path3).is_err()); // original files deleted
        let trashed_path = item.id;
        assert!(!File::open(&trashed_path).is_err()); // returned trash items exist
        std::fs::remove_file(&trashed_path).unwrap(); // clean   up
        assert!(File::open(&trashed_path).is_err()); // cleaned up trash items
    }
}
