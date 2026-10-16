use super::*;

#[test]
fn register_all_is_idempotent() {
    let options = InstallOptions::default();
    register_all(&options);
    register_all(&options);
}
