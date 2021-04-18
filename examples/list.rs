use trash;

#[cfg(not(any(target_os = "windows", all(unix, not(target_os = "macos")))))]
fn main() {
    println!("This is currently only supported on Windows, Linux, and other Freedesktop.org compliant OSes");
}

#[cfg(any(target_os = "windows", all(unix, not(target_os = "macos"))))]
fn main() {
    let trash_items = trash::os_limited::list().unwrap();
    println!("{:#?}", trash_items);
}
