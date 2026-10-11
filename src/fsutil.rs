//! Writing a user's file in place.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Replace the file at `path` with `contents`, all at once, keeping what the
/// file was.
///
/// The contents go to a temporary file beside it, which is synced and then
/// renamed over the original, so a crash or a full disk leaves either the old
/// file or the new one and never half of each. Around that:
///
/// - A symlink is written through to the file it names. Renaming over the
///   link itself would replace it with a regular file, which breaks a config
///   kept in a dotfiles repository.
/// - The original's permissions are kept. The temporary file is created
///   owner-only, so without this every save turned a 0644 note into 0600.
/// - The directory is synced after the rename, so the rename itself survives
///   a crash.
pub(crate) fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    let target = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let dir = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));

    let mut temp = tempfile::NamedTempFile::new_in(dir)?;
    temp.write_all(contents)?;
    if let Ok(original) = fs::metadata(&target) {
        fs::set_permissions(temp.path(), original.permissions())?;
    }
    temp.as_file().sync_all()?;
    temp.persist(&target).map_err(|error| error.error)?;

    #[cfg(unix)]
    if let Ok(dir) = fs::File::open(dir) {
        let _ = dir.sync_all();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_the_contents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "old").unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        // Nothing is left behind beside it.
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn creates_a_file_that_does_not_exist_yet() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        write_atomic(&path, b"x = 1").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "x = 1");
    }

    #[cfg(unix)]
    #[test]
    fn keeps_the_original_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

        write_atomic(&path, b"new").unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o644);
    }

    #[cfg(unix)]
    #[test]
    fn writes_through_a_symlink_rather_than_replacing_it() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("dotfiles-config.toml");
        let link = dir.path().join("config.toml");
        fs::write(&real, "old").unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();

        write_atomic(&link, b"new").unwrap();
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&real).unwrap(), "new");
    }
}
