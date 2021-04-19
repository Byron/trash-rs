#[cfg(not(any(target_os = "windows", all(unix, not(target_os = "macos")))))]
fn main() {
    println!("This is currently only supported on Windows, Linux, and other Freedesktop.org compliant OSes");
}

#[cfg(any(target_os = "windows", all(unix, not(target_os = "macos"))))]
fn main() {
    use chrono::{DateTime, Local, NaiveDateTime, Utc};
    let trash_items = trash::os_limited::list().unwrap();

    let now = Local::now();
    let long_time_ago = now - chrono::Duration::days(42);
    let old_count = trash_items
        .iter()
        .filter(|item| {
            let naive_deletion_utc = NaiveDateTime::from_timestamp(item.time_deleted, 0);
            let deletion = DateTime::<Utc>::from_utc(naive_deletion_utc, Utc);
            deletion < long_time_ago
        })
        .count();

    println!("There are {} old items in your trash.", old_count);
}
