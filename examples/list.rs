use trash;

fn main() {
    println!("This is currently only supported on Linux");
}

#[cfg(target_os = "linux")]
fn main() {
    let trash_items = trash::extra::list().unwrap();
    println!("{:#?}", trash_items);
}
