#[allow(deprecated)]
use crate::{delete, delete_all, remove, remove_all};
use std::fs::{create_dir, File};
use std::path::PathBuf;

#[test]
#[allow(deprecated)]
fn test_remove_file() {
	let path = "test_file_to_remove";
	File::create(path).unwrap();

	remove(path).unwrap();
	assert!(File::open(path).is_err());
}

#[test]
#[allow(deprecated)]
fn test_remove_folder() {
	let path = PathBuf::from("test_folder_to_remove");
	create_dir(&path).unwrap();
	File::create(path.join("file_in_folder")).unwrap();

	assert!(path.exists());
	remove(&path).unwrap();
	assert!(path.exists() == false);
}

#[test]
#[allow(deprecated)]
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

#[test]
fn test_delete_file() {
	let path = "test_file_to_delete";
	File::create(path).unwrap();

	delete(path).unwrap();
	assert!(File::open(path).is_err());
}

#[test]
fn test_delete_folder() {
	let path = PathBuf::from("test_folder_to_delete");
	create_dir(&path).unwrap();
	File::create(path.join("file_in_folder")).unwrap();

	assert!(path.exists());
	delete(&path).unwrap();
	assert!(path.exists() == false);
}

#[test]
fn test_delete_all() {
	let count: usize = 3;

	let paths: Vec<_> = (0..count).map(|i| format!("test_file_to_delete_{}", i)).collect();
	for path in paths.iter() {
		File::create(path).unwrap();
	}

	delete_all(&paths).unwrap();
	for path in paths.iter() {
		assert!(File::open(path).is_err());
	}
}

#[cfg(unix)]
mod unix {
	#[allow(deprecated)]
	use crate::{delete, remove};
	use std::os::unix::fs::symlink;
	use std::fs::{File, create_dir, remove_file, remove_dir_all};

	use std::path::Path;

	#[test]
	#[allow(deprecated)]
	fn test_remove_symlink() {
		let target_path = "test_link_target_for_remove";
		File::create(target_path).unwrap();

		let link_path = Path::new("test_link_to_remove");
		symlink(target_path, link_path).unwrap();

		remove(link_path).unwrap();
		assert!(link_path.symlink_metadata().unwrap().file_type().is_symlink());
		assert!(File::open(target_path).is_err());
		// Cleanup
		remove_file(link_path).unwrap();
	}

	#[test]
	fn test_delete_symlink() {
		let target_path = "test_link_target_for_delete";
		File::create(target_path).unwrap();

		let link_path = "test_link_to_delete";
		symlink(target_path, link_path).unwrap();

		delete(link_path).unwrap();
		assert!(File::open(link_path).is_err());
		assert!(File::open(target_path).is_ok());
		// Cleanup
		remove_file(target_path).unwrap();
	}

	#[test]
	fn test_delete_symlink_in_folder() {
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
	}
}
