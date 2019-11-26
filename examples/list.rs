use trash;

fn main() {
    let trash_items = trash::linux_windows::list().unwrap();
    println!("{:#?}", trash_items);
}
