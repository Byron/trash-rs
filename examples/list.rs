use trash;
use trash::TrashItem;

fn main() {
    let list = trash::list().unwrap();

    for item in list.iter() {
        println!("------------------------------------------------");
        println!("{}", item.name);
        println!("{}", item.original_path().to_str().unwrap());
        println!("{}", item.time_deleted);
    }
}
