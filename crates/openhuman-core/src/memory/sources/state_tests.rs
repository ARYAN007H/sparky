use super::*;

#[test]
fn corrupt_source_state_file_is_set_aside() {
    let tmp = tempfile::tempdir().unwrap();
    let file = path(tmp.path());
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, b"{\"source-1\":").unwrap();

    assert!(load(tmp.path()).is_empty());
    assert!(
        !file.exists(),
        "corrupt state must not be silently replaced"
    );
    assert_eq!(
        std::fs::read(file.with_extension("json.corrupt")).unwrap(),
        b"{\"source-1\":"
    );
}
