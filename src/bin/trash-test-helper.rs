/// Minimal CLI wrapper used by the freedesktop container tests.
///
/// Usage: trash-test-helper delete <path>
///
/// Exits 0 on success, 1 on trash error, 2 on bad arguments.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1] != "delete" {
        eprintln!("Usage: trash-test-helper delete <path>");
        std::process::exit(2);
    }
    match trash::delete(&args[2]) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Error: {e:?}");
            std::process::exit(1);
        }
    }
}
