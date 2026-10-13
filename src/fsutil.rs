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
/// - A read-only file is refused. Renaming over it worked whenever the
///   directory was writable, so the protection did nothing.
///
/// Two cases are written in place instead, without the atomicity, because
/// a rename would do worse:
///
/// - A file with other hard links. A rename gives this name a new file and
///   leaves every other name holding the old contents.
/// - A file whose directory takes no new files, such as a read-only
///   directory or a file bind-mounted on its own into a container, where the
///   temporary file cannot be made or the rename fails across mounts.
pub(crate) fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    let target = resolve_symlinks(path);
    let original = fs::metadata(&target).ok();
    if original
        .as_ref()
        .is_some_and(|meta| meta.permissions().readonly())
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{} is read-only", target.display()),
        ));
    }
    if original.as_ref().is_some_and(has_other_links) {
        return write_in_place(&target, contents);
    }

    match replace(&target, contents, original.as_ref()) {
        Err(error)
            if original.is_some()
                && matches!(
                    error.kind(),
                    io::ErrorKind::PermissionDenied
                        | io::ErrorKind::CrossesDevices
                        | io::ErrorKind::ResourceBusy
                ) =>
        {
            write_in_place(&target, contents)
        }
        result => result,
    }
}

/// Write `contents` to a temporary file beside `target` and rename it over.
fn replace(target: &Path, contents: &[u8], original: Option<&fs::Metadata>) -> io::Result<()> {
    let dir = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));

    let mut temp = tempfile::NamedTempFile::new_in(dir)?;
    temp.write_all(contents)?;
    if let Some(original) = original {
        fs::set_permissions(temp.path(), original.permissions())?;
    }
    temp.as_file().sync_all()?;
    temp.persist(target).map_err(|error| error.error)?;

    #[cfg(unix)]
    if let Ok(dir) = fs::File::open(dir) {
        let _ = dir.sync_all();
    }
    Ok(())
}

/// Overwrite the file at `target` with `contents`, keeping the file itself.
fn write_in_place(target: &Path, contents: &[u8]) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(target)?;
    file.write_all(contents)?;
    file.sync_all()
}

/// The file `path` names, past any symlinks, even when the last one points
/// at nothing yet. A dangling link was renamed over and so replaced by a
/// regular file; writing creates the file it points to, as `>` does.
fn resolve_symlinks(path: &Path) -> std::path::PathBuf {
    if let Ok(target) = fs::canonicalize(path) {
        return target;
    }
    let mut target = path.to_path_buf();
    // The limit Linux puts on a chain of links, which also ends a loop.
    for _ in 0..40 {
        match fs::read_link(&target) {
            Ok(next) => {
                target = match target.parent() {
                    Some(dir) => dir.join(next),
                    None => next,
                }
            }
            Err(_) => break,
        }
    }
    target
}

#[cfg(unix)]
fn has_other_links(meta: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    meta.nlink() > 1
}

#[cfg(not(unix))]
fn has_other_links(_meta: &fs::Metadata) -> bool {
    false
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

    /// A dangling symlink was renamed over, so it became a regular file.
    #[cfg(unix)]
    #[test]
    fn creates_the_file_a_dangling_symlink_names() {
        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join("config.toml");
        fs::create_dir(dir.path().join("dotfiles")).unwrap();
        std::os::unix::fs::symlink("dotfiles/config.toml", &link).unwrap();

        write_atomic(&link, b"new").unwrap();
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("dotfiles/config.toml")).unwrap(),
            "new"
        );
    }

    /// A rename gave this name a new file, and the other name kept the old
    /// contents.
    #[cfg(unix)]
    #[test]
    fn a_hard_linked_file_stays_linked() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        let other = dir.path().join("also.md");
        fs::write(&path, "old").unwrap();
        fs::hard_link(&path, &other).unwrap();

        write_atomic(&path, b"new").unwrap();
        assert_eq!(fs::read_to_string(&other).unwrap(), "new");
    }

    /// Renaming over a read-only file worked whenever the directory was
    /// writable.
    #[test]
    fn a_read_only_file_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "old").unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions).unwrap();

        let error = write_atomic(&path, b"new").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(fs::read_to_string(&path).unwrap(), "old");

        // Let the temporary directory be removed on Windows.
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(&path, permissions).unwrap();
    }

    /// In a directory that takes no new files, the file can still be
    /// written. Making the temporary file beside it failed the save.
    #[cfg(unix)]
    #[test]
    fn a_file_in_a_read_only_directory_is_written_in_place() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "old").unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o555)).unwrap();

        let result = write_atomic(&path, b"new");
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
        // Root writes anywhere, so the rename succeeds there too.
        result.unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
    }
}
