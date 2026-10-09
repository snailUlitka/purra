use std::ffi::{OsStr, OsString};
use std::fs::{self, Permissions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use tempfile::NamedTempFile;
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    Binary,
    InvalidUtf8,
    SymbolicLink,
    UnsupportedFileType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextFile {
    Text(String),
    Skipped(SkipReason),
}

#[derive(Debug, Error)]
pub enum FileError {
    #[error("failed to {operation} {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to walk directory {root}: {source}")]
    Walk {
        root: PathBuf,
        #[source]
        source: walkdir::Error,
    },
    #[error("path has no file name: {0}")]
    MissingFileName(PathBuf),
    #[error("refusing to replace symbolic link: {0}")]
    OutputIsSymbolicLink(PathBuf),
}

/// Options for directory discovery with gitignore-style exclusions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryOptions {
    /// Descend into subdirectories. Disabled by default.
    pub recursive: bool,
    /// Read `.gitignore` in the root and visited subdirectories. Enabled by default.
    pub respect_gitignore: bool,
    /// Additional exclusion globs relative to the scan root. Negation is rejected.
    pub ignores: Vec<String>,
}

impl Default for DirectoryOptions {
    fn default() -> Self {
        Self {
            recursive: false,
            respect_gitignore: true,
            ignores: Vec::new(),
        }
    }
}

/// Errors from filtered directory discovery, separate from the legacy file API.
#[derive(Debug, Error)]
pub enum DirectoryError {
    #[error(transparent)]
    File(#[from] FileError),
    #[error("invalid ignore pattern in {origin}: {source}")]
    InvalidPattern {
        origin: String,
        #[source]
        source: ignore::Error,
    },
    #[error("--ignore only accepts exclusions, not negation: {0}")]
    NegatedIgnore(String),
}

/// Reads a regular UTF-8 text file after classifying unsafe or unsupported
/// inputs. Binary files are detected by a NUL byte.
pub fn read_text(path: impl AsRef<Path>) -> Result<TextFile, FileError> {
    let path = path.as_ref();
    let metadata = fs::symlink_metadata(path).map_err(|source| FileError::Io {
        operation: "inspect",
        path: path.to_path_buf(),
        source,
    })?;

    if metadata.file_type().is_symlink() {
        return Ok(TextFile::Skipped(SkipReason::SymbolicLink));
    }
    if !metadata.is_file() {
        return Ok(TextFile::Skipped(SkipReason::UnsupportedFileType));
    }

    let bytes = fs::read(path).map_err(|source| FileError::Io {
        operation: "read",
        path: path.to_path_buf(),
        source,
    })?;
    if bytes.contains(&0) {
        return Ok(TextFile::Skipped(SkipReason::Binary));
    }

    match String::from_utf8(bytes) {
        Ok(text) => Ok(TextFile::Text(text)),
        Err(_) => Ok(TextFile::Skipped(SkipReason::InvalidUtf8)),
    }
}

/// Collects regular files and symbolic links in deterministic path order.
/// Generated Purra backup files are excluded. This legacy API does not apply
/// `.gitignore`; use [`collect_directory_files_with_options`] for filtering.
pub fn collect_directory_files(
    root: impl AsRef<Path>,
    recursive: bool,
) -> Result<Vec<PathBuf>, FileError> {
    let root = root.as_ref();
    let mut walker = WalkDir::new(root).min_depth(1).follow_links(false);
    if !recursive {
        walker = walker.max_depth(1);
    }

    let mut paths = Vec::new();
    for entry in walker {
        let entry = entry.map_err(|source| FileError::Walk {
            root: root.to_path_buf(),
            source,
        })?;
        let file_type = entry.file_type();
        if (file_type.is_file() || file_type.is_symlink())
            && !is_generated_backup(entry.file_name())
        {
            paths.push(entry.into_path());
        }
    }
    paths.sort();
    Ok(paths)
}

/// Collects files in deterministic path order after validating all applicable
/// ignore rules. Excluded directories are pruned before loading their rules.
/// Only `.gitignore` files inside the scan root are loaded, even outside Git
/// repositories. Hidden files remain eligible and symbolic links are not followed.
pub fn collect_directory_files_with_options(
    root: impl AsRef<Path>,
    options: &DirectoryOptions,
) -> Result<Vec<PathBuf>, DirectoryError> {
    let root = root.as_ref();
    let mut builder = GitignoreBuilder::new(root);
    for pattern in &options.ignores {
        if pattern.starts_with('!') {
            return Err(DirectoryError::NegatedIgnore(pattern.clone()));
        }
        builder
            .add_line(None, pattern)
            .map_err(|source| DirectoryError::InvalidPattern {
                origin: format!("--ignore {pattern:?}"),
                source,
            })?;
    }
    let explicit = builder
        .build()
        .map_err(|source| DirectoryError::InvalidPattern {
            origin: "--ignore arguments".to_owned(),
            source,
        })?;

    let mut walker = WalkDir::new(root)
        .follow_links(false)
        .follow_root_links(false);
    if !options.recursive {
        walker = walker.max_depth(1);
    }
    let mut walker = walker.into_iter();
    let mut scopes: Vec<(usize, Gitignore)> = Vec::new();
    let mut paths = Vec::new();
    while let Some(entry) = walker.next() {
        let entry = entry.map_err(|source| FileError::Walk {
            root: root.to_path_buf(),
            source,
        })?;
        let depth = entry.depth();
        while scopes.last().is_some_and(|(level, _)| *level >= depth) {
            scopes.pop();
        }
        let file_type = entry.file_type();
        let is_dir = file_type.is_dir();
        // The root establishes the rule scope; patterns only match its children.
        if depth != 0 {
            let excluded = explicit.matched(entry.path(), is_dir).is_ignore()
                || scopes
                    .iter()
                    .rev()
                    .map(|(_, rules)| rules.matched(entry.path(), is_dir))
                    .find(|matched| !matched.is_none())
                    .is_some_and(|matched| matched.is_ignore());
            if excluded {
                if is_dir {
                    walker.skip_current_dir();
                }
                continue;
            }
        }

        if is_dir {
            if options.respect_gitignore
                && (depth == 0 || options.recursive)
                && let Some(rules) = load_gitignore(entry.path())?
            {
                scopes.push((depth, rules));
            }
        } else if depth != 0
            && (file_type.is_file() || file_type.is_symlink())
            && !is_generated_backup(entry.file_name())
        {
            paths.push(entry.into_path());
        }
    }
    paths.sort();
    Ok(paths)
}

fn load_gitignore(directory: &Path) -> Result<Option<Gitignore>, DirectoryError> {
    let path = directory.join(".gitignore");
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(FileError::Io {
                operation: "inspect ignore file",
                path,
                source,
            }
            .into());
        }
    };
    if metadata.file_type().is_symlink() {
        return Ok(None);
    }
    if !metadata.is_file() {
        return Err(FileError::Io {
            operation: "read ignore file",
            path,
            source: io::Error::new(io::ErrorKind::InvalidInput, "not a regular file"),
        }
        .into());
    }
    let file = fs::File::open(&path).map_err(|source| FileError::Io {
        operation: "read ignore file",
        path: path.clone(),
        source,
    })?;
    let mut builder = GitignoreBuilder::new(directory);
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|source| FileError::Io {
            operation: "read ignore file",
            path: path.clone(),
            source,
        })?;
        builder
            .add_line(Some(path.clone()), &line)
            .map_err(|source| DirectoryError::InvalidPattern {
                origin: format!("{}:{}", path.display(), index + 1),
                source,
            })?;
    }
    builder
        .build()
        .map(Some)
        .map_err(|source| DirectoryError::InvalidPattern {
            origin: path.display().to_string(),
            source,
        })
}

/// Atomically writes `contents` to `path` without creating a backup.
/// Permissions are copied from the existing destination, or from
/// `permissions_source` when the destination does not exist.
pub fn write_atomic(
    path: impl AsRef<Path>,
    contents: &str,
    permissions_source: Option<&Path>,
) -> Result<(), FileError> {
    let path = path.as_ref();
    reject_symlink_output(path)?;
    let permissions = match existing_permissions(path)? {
        Some(permissions) => Some(permissions),
        None => match permissions_source {
            Some(source) => existing_permissions(source)?,
            None => None,
        },
    };
    persist_temp(path, contents, permissions)
}

/// Creates a uniquely named sibling backup and atomically replaces `path`.
/// The original permissions are applied to the replacement.
pub fn replace_in_place(path: impl AsRef<Path>, contents: &str) -> Result<PathBuf, FileError> {
    let path = path.as_ref();
    reject_symlink_output(path)?;
    let permissions = fs::metadata(path)
        .map_err(|source| FileError::Io {
            operation: "inspect permissions for",
            path: path.to_path_buf(),
            source,
        })?
        .permissions();

    let temporary = prepare_temp(path, contents, Some(permissions))?;
    let backup = next_backup_path(path)?;
    fs::copy(path, &backup).map_err(|source| FileError::Io {
        operation: "create backup for",
        path: path.to_path_buf(),
        source,
    })?;

    temporary.persist(path).map_err(|error| FileError::Io {
        operation: "atomically replace",
        path: path.to_path_buf(),
        source: error.error,
    })?;
    Ok(backup)
}

fn persist_temp(
    path: &Path,
    contents: &str,
    permissions: Option<Permissions>,
) -> Result<(), FileError> {
    let temporary = prepare_temp(path, contents, permissions)?;
    temporary.persist(path).map_err(|error| FileError::Io {
        operation: "atomically write",
        path: path.to_path_buf(),
        source: error.error,
    })?;
    Ok(())
}

fn prepare_temp(
    path: &Path,
    contents: &str,
    permissions: Option<Permissions>,
) -> Result<NamedTempFile, FileError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = NamedTempFile::new_in(parent).map_err(|source| FileError::Io {
        operation: "create temporary file for",
        path: path.to_path_buf(),
        source,
    })?;
    temporary
        .write_all(contents.as_bytes())
        .and_then(|()| temporary.flush())
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|source| FileError::Io {
            operation: "write temporary file for",
            path: path.to_path_buf(),
            source,
        })?;

    if let Some(permissions) = permissions {
        temporary
            .as_file()
            .set_permissions(permissions)
            .map_err(|source| FileError::Io {
                operation: "preserve permissions for",
                path: path.to_path_buf(),
                source,
            })?;
    }
    Ok(temporary)
}

fn reject_symlink_output(path: &Path) -> Result<(), FileError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(FileError::OutputIsSymbolicLink(path.to_path_buf()))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(FileError::Io {
            operation: "inspect",
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn existing_permissions(path: &Path) -> Result<Option<Permissions>, FileError> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(Some(metadata.permissions())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(FileError::Io {
            operation: "inspect permissions for",
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn next_backup_path(path: &Path) -> Result<PathBuf, FileError> {
    let file_name = path
        .file_name()
        .ok_or_else(|| FileError::MissingFileName(path.to_path_buf()))?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));

    for sequence in 0_u64.. {
        let mut backup_name = OsString::from(".");
        backup_name.push(file_name);
        backup_name.push(".purra.bak");
        if sequence != 0 {
            backup_name.push(format!(".{sequence}"));
        }
        let candidate = parent.join(backup_name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    unreachable!("the backup sequence is effectively unbounded")
}

fn is_generated_backup(file_name: &OsStr) -> bool {
    let Some(name) = file_name.to_str() else {
        return false;
    };
    let Some(name) = name.strip_prefix('.') else {
        return false;
    };
    let Some((original, suffix)) = name.rsplit_once(".purra.bak") else {
        return false;
    };
    !original.is_empty()
        && (suffix.is_empty()
            || suffix.strip_prefix('.').is_some_and(|sequence| {
                !sequence.is_empty() && sequence.chars().all(|c| c.is_ascii_digit())
            }))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};

    use tempfile::tempdir;

    use super::{
        SkipReason, TextFile, collect_directory_files, read_text, replace_in_place, write_atomic,
    };

    #[test]
    fn classifies_text_binary_invalid_utf8_and_symlink() {
        let directory = tempdir().unwrap();
        let text = directory.path().join("text.txt");
        let binary = directory.path().join("binary.bin");
        let invalid = directory.path().join("invalid.txt");
        let link = directory.path().join("link.txt");
        fs::write(&text, "hello").unwrap();
        fs::write(&binary, b"a\0b").unwrap();
        fs::write(&invalid, [0xff]).unwrap();
        symlink(&text, &link).unwrap();

        assert_eq!(read_text(&text).unwrap(), TextFile::Text("hello".into()));
        assert_eq!(
            read_text(&binary).unwrap(),
            TextFile::Skipped(SkipReason::Binary)
        );
        assert_eq!(
            read_text(&invalid).unwrap(),
            TextFile::Skipped(SkipReason::InvalidUtf8)
        );
        assert_eq!(
            read_text(&link).unwrap(),
            TextFile::Skipped(SkipReason::SymbolicLink)
        );
    }

    #[test]
    fn atomically_replaces_with_unique_backups_and_preserves_permissions() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("document.txt");
        fs::write(&path, "before").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();

        let first_backup = replace_in_place(&path, "after one").unwrap();
        let second_backup = replace_in_place(&path, "after two").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "after two");
        assert_eq!(fs::read_to_string(first_backup).unwrap(), "before");
        assert_eq!(fs::read_to_string(second_backup).unwrap(), "after one");
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[test]
    fn writes_output_atomically_with_source_permissions() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("source.txt");
        let output = directory.path().join("output.txt");
        fs::write(&source, "source").unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).unwrap();

        write_atomic(&output, "output", Some(&source)).unwrap();

        assert_eq!(fs::read_to_string(&output).unwrap(), "output");
        assert_eq!(
            fs::metadata(output).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn refuses_to_replace_an_output_symlink() {
        let directory = tempdir().unwrap();
        let target = directory.path().join("target.txt");
        let link = directory.path().join("link.txt");
        fs::write(&target, "target").unwrap();
        symlink(&target, &link).unwrap();

        assert!(write_atomic(&link, "replacement", None).is_err());
        assert_eq!(fs::read_to_string(target).unwrap(), "target");
    }

    #[test]
    fn directory_collection_is_non_recursive_by_default_and_skips_backups() {
        let directory = tempdir().unwrap();
        let top = directory.path().join("top.txt");
        let hidden = directory.path().join(".hidden.txt");
        let backup = directory.path().join(".top.txt.purra.bak");
        let regular_with_backup_suffix = directory.path().join("notes.purra.bak");
        let nested_directory = directory.path().join("nested");
        let nested = nested_directory.join("nested.txt");
        fs::create_dir(&nested_directory).unwrap();
        for path in [&top, &hidden, &backup, &regular_with_backup_suffix, &nested] {
            fs::write(path, "text").unwrap();
        }

        let flat = collect_directory_files(directory.path(), false).unwrap();
        let recursive = collect_directory_files(directory.path(), true).unwrap();

        assert_eq!(
            flat,
            vec![hidden, regular_with_backup_suffix.clone(), top.clone()]
        );
        assert_eq!(
            recursive,
            vec![
                directory.path().join(".hidden.txt"),
                nested,
                regular_with_backup_suffix,
                top
            ]
        );
    }
}
