use trash;

fn main() {
    let list = trash::linux_windows::list().unwrap();

    for item in list.iter() {
        println!("------------------------------------------------");
        println!("{}", item.name);
        println!("{}", item.original_path().to_str().unwrap());
        println!("{}", item.time_deleted);
    }
}
