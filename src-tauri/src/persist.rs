// persist.rs — crash-safer JSON writes (temp file in same directory + atomic replace).
// Used by settings, notes, dock, and geometry stores.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Safely write `contents` to `path` using a temporary file in the same directory,
/// flushing buffers to storage, and atomically replacing the destination without
/// deleting the valid copy first.
pub fn atomic_write(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|e| format!("create dir {}: {e}", parent.display()))?;

    // Clean stale temp files if any were left by a previous crash
    let _ = clean_stale_temp_files(parent);

    // Create temporary file in the exact same directory/volume so atomic rename/replace works
    let mut temp = tempfile::Builder::new()
        .prefix(".windash-tmp-")
        .suffix(".json")
        .tempfile_in(parent)
        .map_err(|e| format!("create tempfile in {}: {e}", parent.display()))?;

    temp.write_all(contents.as_bytes())
        .map_err(|e| format!("write tempfile: {e}"))?;
    temp.flush().map_err(|e| format!("flush tempfile: {e}"))?;
    // Flush file buffers to physical storage before replacement
    temp.as_file()
        .sync_all()
        .map_err(|e| format!("sync tempfile: {e}"))?;

    // Atomic replacement: on Windows, NamedTempFile::persist calls MoveFileExW with
    // MOVEFILE_REPLACE_EXISTING, overwriting `path` without an intermediate state
    // where `path` is deleted. If replacement fails, the original destination
    // remains completely intact.
    temp.persist(path)
        .map_err(|e| format!("persist replace {}: {e}", path.display()))?;

    Ok(())
}

/// Remove stale temporary files (`.windash-tmp-*.json`) older than 300 seconds in `dir`.
pub fn clean_stale_temp_files(dir: &Path) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    let now = SystemTime::now();
    for entry in fs::read_dir(dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name.starts_with(".windash-tmp-") && name.ends_with(".json") {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if let Ok(elapsed) = now.duration_since(modified) {
                        if elapsed.as_secs() > 300 {
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Backup a corrupted user data file before restoring defaults.
/// Preserves the corrupted file as `<stem>.corrupt-<timestamp>.bak`.
/// Ensures at most `max_backups` (5) are kept per store by cleaning the oldest.
/// Logs the backup location and returns the backup path if created.
pub fn backup_corrupt_file(path: &Path) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }
    let metadata = fs::metadata(path).ok()?;
    if metadata.len() == 0 {
        return None;
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_stem = path.file_stem()?.to_string_lossy();

    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S%.3f").to_string();
    let backup_name = format!("{file_stem}.corrupt-{timestamp}.bak");
    let mut backup_path = parent.join(&backup_name);

    // Ensure we don't overwrite an existing backup
    let mut counter = 1;
    while backup_path.exists() {
        let alt_name = format!("{file_stem}.corrupt-{timestamp}-{counter}.bak");
        backup_path = parent.join(alt_name);
        counter += 1;
    }

    if let Err(e) = fs::copy(path, &backup_path) {
        log::error!(
            "Failed to backup corrupt file {} to {}: {e}",
            path.display(),
            backup_path.display()
        );
        return None;
    }

    log::warn!(
        "Preserved corrupt data file {} as backup: {}",
        path.display(),
        backup_path.display()
    );

    // Bound the number of backups (keep at most 5)
    bound_backups(parent, &file_stem, 5);

    Some(backup_path)
}

fn bound_backups(dir: &Path, file_stem: &str, max_backups: usize) {
    let prefix = format!("{file_stem}.corrupt-");
    let mut backups: Vec<(PathBuf, SystemTime)> = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&prefix) && name.ends_with(".bak") {
                let mtime = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                backups.push((entry.path(), mtime));
            }
        }
    }

    if backups.len() > max_backups {
        // Sort oldest first
        backups.sort_by_key(|b| b.1);
        let excess = backups.len() - max_backups;
        for (old_path, _) in backups.into_iter().take(excess) {
            let _ = fs::remove_file(old_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::UNIX_EPOCH;

    fn test_dir() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!("windash-test-{nonce}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_atomic_write_creates_new_file() {
        let dir = test_dir();
        let path = dir.join("test_create.json");
        atomic_write(&path, "{\"hello\": \"world\"}").unwrap();
        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"hello\": \"world\"}");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_atomic_write_replaces_existing_file() {
        let dir = test_dir();
        let path = dir.join("test_replace.json");
        atomic_write(&path, "{\"v\": 1}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"v\": 1}");

        atomic_write(&path, "{\"v\": 2}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"v\": 2}");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_resulting_file_is_complete() {
        let dir = test_dir();
        let path = dir.join("test_complete.json");
        let large_content = "X".repeat(50_000);
        atomic_write(&path, &large_content).unwrap();
        let read_back = fs::read_to_string(&path).unwrap();
        assert_eq!(read_back.len(), 50_000);
        assert_eq!(read_back, large_content);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_repeated_replacement() {
        let dir = test_dir();
        let path = dir.join("test_repeated.json");
        for i in 0..10 {
            let content = format!("{{\"iteration\": {i}}}");
            atomic_write(&path, &content).unwrap();
            assert_eq!(fs::read_to_string(&path).unwrap(), content);
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_corrupt_file_backup_preserves_bytes_and_bounds() {
        let dir = test_dir();
        let path = dir.join("windash-notes.json");
        let corrupt_data = "{corrupted json data !!###";
        fs::write(&path, corrupt_data).unwrap();

        // Perform 7 backups to test bounded rotation (max 5)
        for _ in 0..7 {
            let backup = backup_corrupt_file(&path).expect("backup created");
            assert!(backup.exists());
            assert_eq!(fs::read_to_string(&backup).unwrap(), corrupt_data);
            std::thread::sleep(std::time::Duration::from_millis(15));
        }

        let backups: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.starts_with("windash-notes.corrupt-") && name.ends_with(".bak")
            })
            .collect();

        assert_eq!(backups.len(), 5, "at most 5 backups retained");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_backup_corrupt_file_empty_or_nonexistent() {
        let dir = test_dir();
        let nonexistent = dir.join("does_not_exist.json");
        assert!(backup_corrupt_file(&nonexistent).is_none());

        let empty = dir.join("empty.json");
        fs::write(&empty, "").unwrap();
        assert!(backup_corrupt_file(&empty).is_none());
        let _ = fs::remove_dir_all(dir);
    }
}
