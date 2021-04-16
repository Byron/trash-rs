use trash;

fn main() {
    let trash_items = trash::extra::list().unwrap();
    println!("{:#?}", trash_items);
}
