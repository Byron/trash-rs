use std::fs::{create_dir, File};
use std::path::PathBuf;

use trash::{remove, remove_all};

#[test]
fn test_remove_file() {
	let path = "test_file_to_remove";
	File::create(path).unwrap();

	remove(path).unwrap();
	assert!(File::open(path).is_err());
}

#[test]
fn test_remove_folder() {
	let path = PathBuf::from("test_folder_to_remove");
	create_dir(&path).unwrap();
	File::create(path.join("file_in_folder")).unwrap();

	assert!(path.exists());
	remove(&path).unwrap();
	assert!(path.exists() == false);
}

#[test]
fn test_remove_all() {
	let count: usize = 3;

	let paths: Vec<_> = (0..count).map(|i| format!("test_file_to_remove_{}", i)).collect();
	for path in paths.iter() {
		File::create(path).unwrap();
	}

	remove_all(&paths).unwrap();
	for path in paths.iter() {
		assert!(File::open(path).is_err());
	}
}
