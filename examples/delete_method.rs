use std::fs::File;
use trash::TrashContext;

#[cfg(target_os = "macos")]
use trash::macos::{DeleteMethod, TrashContextExtMacos};

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("This example is only available on macOS");
}

#[cfg(target_os = "macos")]
fn main() {
    env_logger::init();

    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::NSFileManager);

    let path = "this_file_was_deleted_using_the_ns_file_manager";
    File::create(path).unwrap();
    trash_ctx.delete(path).unwrap();
    assert!(File::open(path).is_err());
}
