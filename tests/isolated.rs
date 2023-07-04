use trash::delete;

#[test]
fn delete_with_empty_path() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::env::set_current_dir(tmp.path()).unwrap();
    assert_eq!(
        delete("").unwrap_err().to_string(),
        "Error during a `trash` operation: CanonicalizePath { original: \"\" }"
    );
}
