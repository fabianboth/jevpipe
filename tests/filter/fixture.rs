use std::fs;

use tempfile::TempDir;

pub(crate) fn files(entries: &[(&str, &[u8])]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (name, content) in entries {
        fs::write(dir.path().join(name), content).unwrap();
    }
    dir
}
