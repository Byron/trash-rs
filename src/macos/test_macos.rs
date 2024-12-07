use std::process::Command;
use std::ffi::OsString;
use crate::into_unknown;
use crate::tests::init_logging;
use crate::Error;
use serial_test::serial;

fn test_run_as_with_finder_cli() -> Result<(), Error> {
    let mut command = Command::new("osascript");

    let script_text = format!(
        r#"
        tell application "Finder"
            set Trash_items to selection
        end tell
        if (class of Trash_items) is not list then -- if only 1 file is deleted, returns a file, not a list
            return                   (POSIX path of (Trash_items as alias))
        end if
        repeat with aFile in Trash_items -- Finder reference
            set contents of aFile to (POSIX path of (aFile as alias)) -- can't get paths of Finder reference, coersion to alias needed
        end repeat
        return Trash_items
                              "#
    );

    let argv: Vec<OsString> = vec!["-e".into(), script_text.into()];
    command.args(argv);

    // Execute command
    let result = command.output().map_err(into_unknown)?;
    let stdout = String::from_utf8_lossy(&result.stdout);
    println!("stdout={stdout:?}");
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        match result.status.code() {
            None => {
                return Err(Error::Unknown {
                    description: format!("The AppleScript exited with error. stderr: {}", stderr),
                })
            }

            Some(code) => {
                return Err(Error::Os {
                    code,
                    description: format!("The AppleScript exited with error. stderr: {}", stderr),
                })
            }
        };
    }
    Ok(())
}

fn test_run_as_with_finder() -> Result<(), Error> {
    use osakit::{Language, Script};
    let script_text = format!(
        r#"
        tell application "Finder"
            set Trash_items to selection
        end tell
        if (class of Trash_items) is not list then -- if only 1 file is deleted, returns a file, not a list
            return                   (POSIX path of (Trash_items as alias))
        end if
        repeat with aFile in Trash_items -- Finder reference
            set contents of aFile to (POSIX path of (aFile as alias)) -- can't get paths of Finder reference, coersion to alias needed
        end repeat
        return Trash_items
                              "#
    );

    let mut script = Script::new_from_source(Language::AppleScript, &script_text);

    // Compile and Execute script
    // println!("  ? script starting...");
    match script.compile() {
        Ok(_) => {
            // println!("  ✓ script compiled");
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
    let ten_millis = time::Duration::from_millis(100);
    for i in 0..10 { println!("{i} sleeping cli"); // thread::sleep(ten_millis);
        test_run_as_with_finder_cli().unwrap();
    }
    for i in 0..10 { println!("{i} sleeping"); thread::sleep(ten_millis);
        test_run_as_with_finder().unwrap();
    }
}
