use crate as trash;
use std::collections::{hash_map::Entry, HashMap};
use std::fs::{create_dir, File};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono;
use lazy_static::lazy_static;

// WARNING Expecting that `cargo test` won't be invoked on the same computer more than once within
// a single millisecond
lazy_static! {
    static ref INSTANCE_ID: i64 = chrono::Local::now().timestamp_millis();
    static ref ID_OFFSET: AtomicI64 = AtomicI64::new(0);
}
fn get_unique_name() -> String {
    let id = ID_OFFSET.fetch_add(1, Ordering::SeqCst);
    format!("trash-test-{}-{}", *INSTANCE_ID, id)
}

#[test]
fn create_remove() {
    let file_name = get_unique_name();
    File::create(&file_name).unwrap();
    trash::remove(&file_name).unwrap();
    assert!(File::open(&file_name).is_err());
}

#[test]
fn create_remove_empty_folder() {
    let folder_name = get_unique_name();
    let path = PathBuf::from(folder_name);
    create_dir(&path).unwrap();

    assert!(path.exists());
    trash::remove(&path).unwrap();
    assert!(path.exists() == false);
}

#[test]
fn create_remove_folder_with_file() {
    let folder_name = get_unique_name();
    let path = PathBuf::from(folder_name);
    create_dir(&path).unwrap();
    File::create(path.join("file_in_folder")).unwrap();

    assert!(path.exists());
    trash::remove(&path).unwrap();
    assert!(path.exists() == false);
}

#[test]
fn create_multiple_remove_all() {
    let file_name_prefix = get_unique_name();
    let count: usize = 3;
    let paths: Vec<_> = (0..count).map(|i| format!("{}#{}", file_name_prefix, i)).collect();
    for path in paths.iter() {
        File::create(path).unwrap();
    }

    trash::remove_all(&paths).unwrap();
    for path in paths.iter() {
        assert!(File::open(path).is_err());
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
mod linux_windows {
    use super::*;

    #[test]
    fn list() {
        let file_name_prefix = get_unique_name();
        let batches: usize = 2;
        let files_per_batch: usize = 3;
        let names: Vec<_> =
            (0..files_per_batch).map(|i| format!("{}#{}", file_name_prefix, i)).collect();
        for _ in 0..batches {
            for path in names.iter() {
                File::create(path).unwrap();
            }
            trash::remove_all(&names).unwrap();
        }
        let items = trash::linux_windows::list().unwrap();
        let items: HashMap<_, Vec<_>> = items
            .into_iter()
            .filter(|x| x.name.starts_with(&file_name_prefix))
            .fold(HashMap::new(), |mut map, x| {
                match map.entry(x.name.clone()) {
                    Entry::Occupied(mut entry) => {
                        entry.get_mut().push(x);
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(vec![x]);
                    }
                }
                map
            });
        for name in names {
            match items.get(&name) {
                Some(items) => assert_eq!(items.len(), batches),
                None => panic!("ERROR Could not find '{}' in {:#?}", name, items),
            }
        }

        // Let's try to purge all the items we just created but ignore any errors
        // as this test should succeed as long as `list` works properly.
        let _ =
            trash::linux_windows::purge_all(items.into_iter().map(|(_name, item)| item).flatten());
    }

    #[test]
    fn purge_empty() {
        trash::linux_windows::purge_all(vec![]).unwrap();
    }

    #[test]
    fn restore_empty() {
        trash::linux_windows::restore_all(vec![]).unwrap();
    }

    #[test]
    fn purge() {
        let file_name_prefix = get_unique_name();
        let batches: usize = 2;
        let files_per_batch: usize = 3;
        let names: Vec<_> =
            (0..files_per_batch).map(|i| format!("{}#{}", file_name_prefix, i)).collect();
        for _ in 0..batches {
            for path in names.iter() {
                File::create(path).unwrap();
            }
            trash::remove_all(&names).unwrap();
        }

        // Collect it because we need the exact number of items gathered.
        let targets: Vec<_> = trash::linux_windows::list()
            .unwrap()
            .into_iter()
            .filter(|x| x.name.starts_with(&file_name_prefix))
            .collect();
        assert_eq!(targets.len(), batches * files_per_batch);
        trash::linux_windows::purge_all(targets).unwrap();
        let remaining = trash::linux_windows::list()
            .unwrap()
            .into_iter()
            .filter(|x| x.name.starts_with(&file_name_prefix))
            .count();
        assert_eq!(remaining, 0);
    }

    #[test]
    fn restore() {
        let file_name_prefix = get_unique_name();
        let file_count: usize = 3;
        let names: Vec<_> =
            (0..file_count).map(|i| format!("{}#{}", file_name_prefix, i)).collect();
        for path in names.iter() {
            File::create(path).unwrap();
        }
        trash::remove_all(&names).unwrap();

        // Collect it because we need the exact number of items gathered.
        let targets: Vec<_> = trash::linux_windows::list()
            .unwrap()
            .into_iter()
            .filter(|x| x.name.starts_with(&file_name_prefix))
            .collect();
        assert_eq!(targets.len(), file_count);
        trash::linux_windows::restore_all(targets).unwrap();
        let remaining = trash::linux_windows::list()
            .unwrap()
            .into_iter()
            .filter(|x| x.name.starts_with(&file_name_prefix))
            .count();
        assert_eq!(remaining, 0);

        // They are not in the trash anymore but they should be at their original location
        for path in names.iter() {
            assert!(File::open(path).is_ok());
        }

        // Good ol' remove to clean up
        for path in names.iter() {
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn restore_collision() {
        let file_name_prefix = get_unique_name();
        let file_count: usize = 3;
        let collision_remaining = file_count - 1;
        let names: Vec<_> =
            (0..file_count).map(|i| format!("{}#{}", file_name_prefix, i)).collect();
        for path in names.iter() {
            File::create(path).unwrap();
        }
        trash::remove_all(&names).unwrap();
        for path in names.iter().skip(file_count - collision_remaining) {
            File::create(path).unwrap();
        }
        let mut targets: Vec<_> = trash::linux_windows::list()
            .unwrap()
            .into_iter()
            .filter(|x| x.name.starts_with(&file_name_prefix))
            .collect();
        targets.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(targets.len(), file_count);
        let remaining_count;
        match trash::linux_windows::restore_all(targets) {
            Err(e) => match e.kind() {
                trash::ErrorKind::RestoreCollision { remaining_items, .. } => {
                    let contains = |v: &Vec<trash::TrashItem>, name: &String| {
                        for curr in v.iter() {
                            if *curr.name == *name {
                                return true;
                            }
                        }
                        false
                    };
                    // Are all items that got restored reside in the folder?
                    for path in names.iter().filter(|filename| !contains(&remaining_items, filename)) {
                        assert!(File::open(path).is_ok());
                    }
                    remaining_count = remaining_items.len();
                }
                _ => panic!("{:?}", e),
            },
            Ok(()) => panic!(
                "restore_all was expected to return `trash::ErrorKind::RestoreCollision` but did not."
            ),
        }
        let remaining = trash::linux_windows::list()
            .unwrap()
            .into_iter()
            .filter(|x| x.name.starts_with(&file_name_prefix))
            .collect::<Vec<_>>();
        assert_eq!(remaining.len(), remaining_count);
        trash::linux_windows::purge_all(remaining).unwrap();
        for path in names.iter() {
            // This will obviously fail on the items that both didn't collide and weren't restored.
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn restore_twins() {
        let file_name_prefix = get_unique_name();
        let file_count: usize = 4;
        let names: Vec<_> =
            (0..file_count).map(|i| format!("{}#{}", file_name_prefix, i)).collect();
        for path in names.iter() {
            File::create(path).unwrap();
        }
        trash::remove_all(&names).unwrap();

        let twin_name = &names[1];
        File::create(twin_name).unwrap();
        trash::remove(&twin_name).unwrap();

        let mut targets: Vec<_> = trash::linux_windows::list()
            .unwrap()
            .into_iter()
            .filter(|x| x.name.starts_with(&file_name_prefix))
            .collect();
        targets.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(targets.len(), file_count + 1); // plus one for one of the twins
        match trash::linux_windows::restore_all(targets) {
            Err(e) => match e.kind() {
                trash::ErrorKind::RestoreTwins { path, .. } => {
                    assert_eq!(path.file_name().unwrap().to_str().unwrap(), twin_name);
                    match e.into_kind() {
                        trash::ErrorKind::RestoreTwins { items, .. } => {
                            trash::linux_windows::purge_all(items).unwrap();
                        }
                        _ => unreachable!(),
                    }
                }
                _ => panic!("{:?}", e),
            },
            Ok(()) => panic!(
                "restore_all was expected to return `trash::ErrorKind::RestoreTwins` but did not."
            ),
        }
    }
}
