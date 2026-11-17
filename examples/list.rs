#[cfg(not(any(
    target_os = "windows",
    all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android"), feature = "freedesktop")
)))]
fn main() {
    println!("This is currently only supported on Windows, Linux, and other Freedesktop.org compliant OSes");
}

#[cfg(any(
    target_os = "windows",
    all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android"), feature = "freedesktop")
))]
fn main() {
    let trash_items = trash::os_limited::list().unwrap();

    let long_time_ago = jiff::Timestamp::now() - jiff::SignedDuration::from_hours(42 * 24);
    let old_count = trash_items.iter().filter(|item| item.time_deleted < long_time_ago.as_second()).count();

    println!("There are {} old items in your trash.", old_count);
}
