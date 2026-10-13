//! A per-test database directory for the store tests.

use std::path::PathBuf;

/// A per-test directory under the system temp dir, removed on drop.
pub struct TempDb(PathBuf);

impl TempDb {
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("bluetti-store-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    pub fn path(&self) -> PathBuf {
        self.0.join("test.db")
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
