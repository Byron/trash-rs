#[cfg(not(any(
    target_os = "windows",
    all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android"))
)))]
fn main() {
    println!("This is currently only supported on Windows, Linux, and other Freedesktop.org compliant OSes");
}

#[cfg(any(
    target_os = "windows",
    all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android"))
))]
fn main() {
    let trash_items = trash::os_limited::list().unwrap();

    for item in trash_items {
        let metadata = trash::os_limited::metadata(&item).unwrap();
        println!("{:?}: {:?}", item, metadata);
    }
}
