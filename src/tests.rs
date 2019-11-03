use crate as trash;
use std::fs::{create_dir, File};
use std::path::PathBuf;
use std::collections::{HashMap, hash_map::Entry};
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
fn create_remove_folder() {
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
    let paths: Vec<_> = (0..count)
        .map(|i| format!("{}#{}", file_name_prefix, i))
        .collect();
    for path in paths.iter() {
        File::create(path).unwrap();
    }

    trash::remove_all(&paths).unwrap();
    for path in paths.iter() {
        assert!(File::open(path).is_err());
    }
}

#[test]
fn list() {
    let file_name_prefix = get_unique_name();
    let batches: usize = 2;
    let files_per_batch: usize = 3;
    let names: Vec<_> = (0..files_per_batch)
        .map(|i| format!("{}#{}", file_name_prefix, i))
        .collect();
    for _ in 0..batches {
        for path in names.iter() {
            File::create(path).unwrap();
        }
        trash::remove_all(&names).unwrap();
    }
    let items = trash::list().unwrap();
    let items: HashMap<_, Vec<_>> =
        items.into_iter().filter(|x| x.name.starts_with(&file_name_prefix))
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
        assert_eq!(items.get(&name).unwrap().len(), batches);
    }

    // Let's try to purge all the items we just created but ignore any errors
    // as this test should succeed as long as `list` works properly.
    let _ = trash::purge_all(items.into_iter().map(|(_name, item)| item).flatten());
}

#[test]
fn purge() {
    let file_name_prefix = get_unique_name();
    let batches: usize = 2;
    let files_per_batch: usize = 3;
    let names: Vec<_> = (0..files_per_batch)
        .map(|i| format!("{}#{}", file_name_prefix, i))
        .collect();
    for _ in 0..batches {
        for path in names.iter() {
            File::create(path).unwrap();
        }
        trash::remove_all(&names).unwrap();
    }

    // Collect it because we need the exact number of items gathered.
    let targets: Vec<_> = trash::list()
        .unwrap()
        .into_iter()
        .filter(|x| x.name.starts_with(&file_name_prefix))
        .collect();
    assert_eq!(targets.len(), batches * files_per_batch);
    trash::purge_all(targets).unwrap();
    // Ugly hack but need to wait for example on Windows one must wait a bit
    // before the items acctually leave the trash
    //std::thread::sleep(std::time::Duration::from_secs(8));
    let remaining = trash::list()
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
    let names: Vec<_> = (0..file_count)
        .map(|i| format!("{}#{}", file_name_prefix, i))
        .collect();
    for path in names.iter() {
        File::create(path).unwrap();
    }
    trash::remove_all(&names).unwrap();

    // Collect it because we need the exact number of items gathered.
    let targets: Vec<_> = trash::list()
        .unwrap()
        .into_iter()
        .filter(|x| x.name.starts_with(&file_name_prefix))
        .collect();
    assert_eq!(targets.len(), file_count);
    trash::restore_all(targets).unwrap();

    // Ugly hack but need to wait for example on Windows one must wait a bit
    // before the items acctually leave the trash
    //std::thread::sleep(std::time::Duration::from_secs(8));

    let remaining = trash::list()
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
