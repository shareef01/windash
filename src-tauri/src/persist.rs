// persist.rs — crash-safer JSON writes (temp file + replace).
// Used by settings, notes, dock, and geometry stores.

use std::fs;
use std::path::Path;

pub fn atomic_write(path: &Path, contents: &str) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, contents).map_err(|e| format!("write: {e}"))?;
    // On Windows, rename fails if the destination exists.
    if path.exists() {
        fs::remove_file(path).map_err(|e| format!("replace: {e}"))?;
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("rename: {e}")
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn atomic_write_creates_and_replaces() {
        let dir = env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = dir.join(format!("windash-persist-{nonce}.json"));
        atomic_write(&path, "{\"a\":1}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":1}");
        atomic_write(&path, "{\"a\":2}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":2}");
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("tmp"));
    }
}
