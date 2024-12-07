use crate::tests::init_logging;
use crate::Error;
use serial_test::serial;

fn test_run_as_with_finder() -> Result<(), Error> {
    use osakit::{Language, Script};
    let script_text = format!(
        r#"
        tell application "Finder"
            set Trash_items to selection
        end tell
        return "empty string"
                              "#
    );

    let mut script = Script::new_from_source(Language::AppleScript, &script_text);

    // Compile and Execute script
    println!("  ? script starting...");
    match script.compile() {
        Ok(_) => {
            println!("  ✓ script compiled");
            match script.execute() {
                Ok(res) => println!("  ✓ script run res={:?}", &res),
                Err(e) => {
                    println!("  ✗ script failed to run");
                    return Err(Error::Unknown { description: format!("The AppleScript failed with error: {}", e) });
                }
            }
        }
        Err(e) => {
            println!("  ✗ script failed to compile");
            return Err(Error::Unknown { description: format!("The AppleScript failed to compile with error: {}", e) });
        }
    }
    Ok(())
}

use std::{thread, time};
#[test]
#[serial]
fn test_x1() {
    init_logging();
    let ten_millis = time::Duration::from_millis(1000);
    for i in 0..10 {
        println!("{i} sleeping");
        thread::sleep(ten_millis);
        test_run_as_with_finder().unwrap();
    }
}
