// Separate file to force running the tests on the main thread, which is required for any macOS OSAkit APIs, which otherwise fail after a 2-min stall
use trash::{
    canonicalize_paths,
    macos::{percent_encode, DeleteMethod, ScriptMethod, TrashContextExtMacos},
    tests::{get_unique_name, init_logging},
    TrashContext,
};
use serial_test::serial;
use std::ffi::OsStr;
use std::fs::File;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::Command;

#[test]
#[serial]
fn test_delete_with_finder_osakit_with_info() { // tested to work, but not always: can randomly timeout, so disabled by default
    init_logging();
    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::Finder);
    trash_ctx.set_script_method(ScriptMethod::Osakit);

    let mut path1 = PathBuf::from(get_unique_name());
    let mut path2 = PathBuf::from(get_unique_name());
    path1.set_extension(r#"a"b,"#);
    path2.set_extension(r#"x80=%80 slash=\ pc=% quote=" comma=,"#);
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
