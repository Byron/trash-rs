use crate::{
    macos::{DeleteMethod, TrashContextExtMacos},
    tests::{get_unique_name, init_logging},
    TrashContext,
};
use serial_test::serial;
use std::path::PathBuf;


#[test] #[serial] fn test_x1 () {
    use std::{thread, time};
    println!("sleeping");
    let ten_millis = time::Duration::from_millis(3000);
    thread::sleep(ten_millis);
    init_logging();
    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::Finder);

    let path1 = PathBuf::from(get_unique_name());
    let path2 = PathBuf::from(get_unique_name());
    trash_ctx.delete_all(&[&path1, &path2]).unwrap();
}

#[test] #[serial] fn test_x2 () {
    use std::{thread, time};
    println!("sleeping");
    let ten_millis = time::Duration::from_millis(3000);
    thread::sleep(ten_millis);
    init_logging();
    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::Finder);

    let path1 = PathBuf::from(get_unique_name());
    let path2 = PathBuf::from(get_unique_name());
    trash_ctx.delete_all(&[&path1, &path2]).unwrap();
}

#[test] #[serial] fn test_x3 () {
    use std::{thread, time};
    println!("sleeping");
    let ten_millis = time::Duration::from_millis(3000);
    thread::sleep(ten_millis);
    init_logging();
    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::Finder);

    let path1 = PathBuf::from(get_unique_name());
    let path2 = PathBuf::from(get_unique_name());
    trash_ctx.delete_all(&[&path1, &path2]).unwrap();
}

#[test] #[serial] fn test_x4 () {
    use std::{thread, time};
    println!("sleeping");
    let ten_millis = time::Duration::from_millis(3000);
    thread::sleep(ten_millis);
    init_logging();
    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::Finder);

    let path1 = PathBuf::from(get_unique_name());
    let path2 = PathBuf::from(get_unique_name());
    trash_ctx.delete_all(&[&path1, &path2]).unwrap();
}

#[test] #[serial] fn test_x5 () {
    use std::{thread, time};
    println!("sleeping");
    let ten_millis = time::Duration::from_millis(3000);
    thread::sleep(ten_millis);
    init_logging();
    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::Finder);

    let path1 = PathBuf::from(get_unique_name());
    let path2 = PathBuf::from(get_unique_name());
    trash_ctx.delete_all(&[&path1, &path2]).unwrap();
}

#[test] #[serial] fn test_x6 () {
    use std::{thread, time};
    println!("sleeping");
    let ten_millis = time::Duration::from_millis(3000);
    thread::sleep(ten_millis);
    init_logging();
    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::Finder);

    let path1 = PathBuf::from(get_unique_name());
    let path2 = PathBuf::from(get_unique_name());
    trash_ctx.delete_all(&[&path1, &path2]).unwrap();
}

#[test] #[serial] fn test_x7 () {
    use std::{thread, time};
    println!("sleeping");
    let ten_millis = time::Duration::from_millis(3000);
    thread::sleep(ten_millis);
    init_logging();
    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::Finder);

    let path1 = PathBuf::from(get_unique_name());
    let path2 = PathBuf::from(get_unique_name());
    trash_ctx.delete_all(&[&path1, &path2]).unwrap();
}
