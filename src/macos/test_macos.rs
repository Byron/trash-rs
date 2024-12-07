use crate::{
    macos::{DeleteMethod, TrashContextExtMacos},
    tests::{get_unique_name, init_logging},
    TrashContext,
};
use serial_test::serial;
use std::path::PathBuf;

use std::{thread, time};
#[test] #[serial] fn test_x1 () {
    init_logging();
    for i in 0..10 {
        println!("{i} sleeping");
        let ten_millis = time::Duration::from_millis(1000);
        thread::sleep(ten_millis);

        let mut trash_ctx = TrashContext::default();
        trash_ctx.set_delete_method(DeleteMethod::Finder);

        let path1 = PathBuf::from(get_unique_name());
        let path2 = PathBuf::from(get_unique_name());
        trash_ctx.delete_all(&[&path1, &path2]).unwrap();
    }
}
