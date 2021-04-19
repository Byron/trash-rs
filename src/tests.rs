use std::fs::{create_dir, File};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono;
use log::trace;
use once_cell::sync::Lazy;

use crate::{delete, delete_all};

// WARNING Expecting that `cargo test` won't be invoked on the same computer more than once within
// a single millisecond
static INSTANCE_ID: Lazy<i64> = Lazy::new(|| chrono::Local::now().timestamp_millis());
static ID_OFFSET: AtomicI64 = AtomicI64::new(0);
pub fn get_unique_name() -> String {
    let id = ID_OFFSET.fetch_add(1, Ordering::SeqCst);
    format!("trash-test-{}-{}", *INSTANCE_ID, id)
}

fn init_logging() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[test]
fn test_delete_file() {
    init_logging();
    trace!("Started test_delete_file");

    let path = get_unique_name();
    File::create(&path).unwrap();

    delete(&path).unwrap();
    assert!(File::open(&path).is_err());
    trace!("Finished test_delete_file");
}

#[test]
fn test_delete_folder() {
    init_logging();
    trace!("Started test_delete_folder");

    let path = PathBuf::from(get_unique_name());
    create_dir(&path).unwrap();
    File::create(path.join("file_in_folder")).unwrap();

    assert!(path.exists());
    delete(&path).unwrap();
    assert!(path.exists() == false);

    trace!("Finished test_delete_folder");
}

#[test]
fn test_delete_all() {
    init_logging();
    trace!("Started test_delete_all");
    let count: usize = 3;

    let paths: Vec<_> = (0..count).map(|i| format!("test_file_to_delete_{}", i)).collect();
    for path in paths.iter() {
        File::create(path).unwrap();
    }

    delete_all(&paths).unwrap();
    for path in paths.iter() {
        assert!(File::open(path).is_err());
    }
    trace!("Finished test_delete_all");
}

#[cfg(unix)]
mod unix {
    use log::trace;
    use std::{
        fs::{create_dir, remove_dir_all, remove_file, File},
        os::unix::fs::symlink,
        path::Path,
    };

    use super::{get_unique_name, init_logging};
    use crate::delete;
    // use crate::init_logging;

    #[test]
    fn test_delete_symlink() {
        init_logging();
        trace!("Started test_delete_symlink");
        let target_path = get_unique_name();
        File::create(&target_path).unwrap();

        let link_path = "test_link_to_delete";
        symlink(&target_path, link_path).unwrap();

        delete(link_path).unwrap();
        assert!(File::open(link_path).is_err());
        assert!(File::open(&target_path).is_ok());
        // Cleanup
        remove_file(&target_path).unwrap();
        trace!("Finished test_delete_symlink");
    }

    #[test]
    fn test_delete_symlink_in_folder() {
        init_logging();
        trace!("Started test_delete_symlink_in_folder");
        let target_path = "test_link_target_for_delete_from_folder";
        File::create(target_path).unwrap();

        let folder = Path::new("test_parent_folder_for_link_to_delete");
        create_dir(folder).unwrap();
        let link_path = folder.join("test_link_to_delete_from_folder");
        symlink(target_path, &link_path).unwrap();

        delete(&link_path).unwrap();
        assert!(File::open(link_path).is_err());
        assert!(File::open(target_path).is_ok());
        // Cleanup
        remove_file(target_path).unwrap();
        remove_dir_all(folder).unwrap();
        trace!("Finished test_delete_symlink_in_folder");
    }
}

#[cfg(any(target_os = "windows", all(unix, not(target_os = "macos"))))]
mod os_limited {
    use std::collections::{hash_map::Entry, HashMap};

    use super::*;

    use crate as trash;

    #[test]
    fn list() {
        const MAX_SECONDS_DIFFERENCE: i64 = 10;
        init_logging();

        let deletion_time = chrono::Utc::now();
        let actual_unix_deletion_time = deletion_time.naive_utc().timestamp();
        assert_eq!(actual_unix_deletion_time, deletion_time.naive_local().timestamp());
        let file_name_prefix = get_unique_name();
        let batches: usize = 2;
        let files_per_batch: usize = 3;
        let names: Vec<_> =
            (0..files_per_batch).map(|i| format!("{}#{}", file_name_prefix, i)).collect();
        for _ in 0..batches {
            for path in names.iter() {
                File::create(path).unwrap();
            }
            trash::delete_all(&names).unwrap();
        }
        let items = trash::os_limited::list().unwrap();
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
                Some(items) => {
                    assert_eq!(items.len(), batches);
                    for item in items {
                        let diff = (item.time_deleted - actual_unix_deletion_time).abs();
                        if diff > MAX_SECONDS_DIFFERENCE {
                            panic!(
                                "The deleted item does not have the timestamp that represents its deletion time. Expected: {}. Got: {}",
                                actual_unix_deletion_time,
                                item.time_deleted
                            );
                        }
                    }
                }
                None => panic!("ERROR Could not find '{}' in {:#?}", name, items),
            }
        }

        // Let's try to purge all the items we just created but ignore any errors
        // as this test should succeed as long as `list` works properly.
        let _ = trash::os_limited::purge_all(items.into_iter().map(|(_name, item)| item).flatten());
    }

    #[test]
    fn purge_empty() {
        init_logging();
        trash::os_limited::purge_all(vec![]).unwrap();
    }

    #[test]
    fn restore_empty() {
        init_logging();
        trash::os_limited::restore_all(vec![]).unwrap();
    }

    #[test]
    fn purge() {
        init_logging();
        let file_name_prefix = get_unique_name();
        let batches: usize = 2;
        let files_per_batch: usize = 3;
        let names: Vec<_> =
            (0..files_per_batch).map(|i| format!("{}#{}", file_name_prefix, i)).collect();
        for _ in 0..batches {
            for path in names.iter() {
                File::create(path).unwrap();
            }
            trash::delete_all(&names).unwrap();
        }

        // Collect it because we need the exact number of items gathered.
        let targets: Vec<_> = trash::os_limited::list()
            .unwrap()
            .into_iter()
            .filter(|x| x.name.starts_with(&file_name_prefix))
            .collect();
        assert_eq!(targets.len(), batches * files_per_batch);
        trash::os_limited::purge_all(targets).unwrap();
        let remaining = trash::os_limited::list()
            .unwrap()
            .into_iter()
            .filter(|x| x.name.starts_with(&file_name_prefix))
            .count();
        assert_eq!(remaining, 0);
    }

    #[test]
    fn restore() {
        init_logging();
        let file_name_prefix = get_unique_name();
        let file_count: usize = 3;
        let names: Vec<_> =
            (0..file_count).map(|i| format!("{}#{}", file_name_prefix, i)).collect();
        for path in names.iter() {
            File::create(path).unwrap();
        }
        trash::delete_all(&names).unwrap();

        // Collect it because we need the exact number of items gathered.
        let targets: Vec<_> = trash::os_limited::list()
            .unwrap()
            .into_iter()
            .filter(|x| x.name.starts_with(&file_name_prefix))
            .collect();
        assert_eq!(targets.len(), file_count);
        trash::os_limited::restore_all(targets).unwrap();
        let remaining = trash::os_limited::list()
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
        init_logging();
        let file_name_prefix = get_unique_name();
        let file_count: usize = 3;
        let collision_remaining = file_count - 1;
        let names: Vec<_> =
            (0..file_count).map(|i| format!("{}#{}", file_name_prefix, i)).collect();
        for path in names.iter() {
            File::create(path).unwrap();
        }
        trash::delete_all(&names).unwrap();
        for path in names.iter().skip(file_count - collision_remaining) {
            File::create(path).unwrap();
        }
        let mut targets: Vec<_> = trash::os_limited::list()
            .unwrap()
            .into_iter()
            .filter(|x| x.name.starts_with(&file_name_prefix))
            .collect();
        targets.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(targets.len(), file_count);
        let remaining_count;
        match trash::os_limited::restore_all(targets) {
            Err(trash::Error::RestoreCollision { remaining_items, .. }) => {
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
            },
            _ => panic!(
                "restore_all was expected to return `trash::ErrorKind::RestoreCollision` but did not."
            ),
        }
        let remaining = trash::os_limited::list()
            .unwrap()
            .into_iter()
            .filter(|x| x.name.starts_with(&file_name_prefix))
            .collect::<Vec<_>>();
        assert_eq!(remaining.len(), remaining_count);
        trash::os_limited::purge_all(remaining).unwrap();
        for path in names.iter() {
            // This will obviously fail on the items that both didn't collide and weren't restored.
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn restore_twins() {
        init_logging();
        let file_name_prefix = get_unique_name();
        let file_count: usize = 4;
        let names: Vec<_> =
            (0..file_count).map(|i| format!("{}#{}", file_name_prefix, i)).collect();
        for path in names.iter() {
            File::create(path).unwrap();
        }
        trash::delete_all(&names).unwrap();

        let twin_name = &names[1];
        File::create(twin_name).unwrap();
        trash::delete(&twin_name).unwrap();

        let mut targets: Vec<_> = trash::os_limited::list()
            .unwrap()
            .into_iter()
            .filter(|x| x.name.starts_with(&file_name_prefix))
            .collect();
        targets.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(targets.len(), file_count + 1); // plus one for one of the twins
        match trash::os_limited::restore_all(targets) {
            Err(trash::Error::RestoreTwins { path, items }) => {
                assert_eq!(path.file_name().unwrap().to_str().unwrap(), twin_name);
                trash::os_limited::purge_all(items).unwrap();
            }
            _ => panic!(
                "restore_all was expected to return `trash::ErrorKind::RestoreTwins` but did not."
            ),
        }
    }
}
