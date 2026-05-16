#![allow(dead_code)]

use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

pub const MAX_BYTES: u64 = 10 * 1024 * 1024;
pub const BINARY_SNIFF_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileEtag {
    pub mtime: i64,
    pub size: u64,
}

impl FileEtag {
    pub fn for_path(path: &Path) -> io::Result<Self> {
        let meta = fs::metadata(path)?;
        Ok(Self {
            mtime: meta.mtime(),
            size: meta.size(),
        })
    }
}

#[derive(Debug, Clone)]
pub enum LoadResult {
    Text { contents: String, etag: FileEtag },
    Binary { etag: FileEtag },
    TooLarge { size: u64 },
    NotFound,
    Io(String),
}

pub fn load(path: &Path) -> LoadResult {
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return LoadResult::NotFound,
        Err(e) => return LoadResult::Io(e.to_string()),
    };
    if meta.size() > MAX_BYTES {
        return LoadResult::TooLarge { size: meta.size() };
    }
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => return LoadResult::Io(e.to_string()),
    };
    let etag = FileEtag {
        mtime: meta.mtime(),
        size: meta.size(),
    };
    let sniff_end = bytes.len().min(BINARY_SNIFF_BYTES);
    if bytes[..sniff_end].contains(&0u8) {
        return LoadResult::Binary { etag };
    }
    match String::from_utf8(bytes) {
        Ok(s) => LoadResult::Text { contents: s, etag },
        Err(_) => LoadResult::Binary { etag },
    }
}

pub fn save_atomic(path: &Path, contents: &str) -> io::Result<FileEtag> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no parent dir"))?;
    let tmp = parent.join(format!(
        ".{}.lyrux-tmp.{}.{}",
        path.file_name().and_then(|s| s.to_str()).unwrap_or("file"),
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    {
        use std::io::Write;
        let mut f = fs::File::create(&tmp)?;
        f.write_all(contents.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    FileEtag::for_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn load_text_returns_contents_and_etag() {
        let dir = tempdir().unwrap();
        let p: PathBuf = dir.path().join("a.rs");
        fs::write(&p, "hello").unwrap();
        match load(&p) {
            LoadResult::Text { contents, etag } => {
                assert_eq!(contents, "hello");
                assert_eq!(etag.size, 5);
            }
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn load_binary_returns_binary_when_null_byte_present() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("img.bin");
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(&[0u8, 1, 2, 3]).unwrap();
        assert!(matches!(load(&p), LoadResult::Binary { .. }));
    }

    #[test]
    fn load_too_large_when_above_cap() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("big.txt");
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(&vec![b'a'; (MAX_BYTES as usize) + 1]).unwrap();
        assert!(matches!(load(&p), LoadResult::TooLarge { .. }));
    }

    #[test]
    fn load_not_found_for_missing_file() {
        assert!(matches!(
            load(Path::new("/nonexistent/lyrux-nope")),
            LoadResult::NotFound
        ));
    }

    #[test]
    fn save_atomic_writes_and_returns_etag() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("out.txt");
        fs::write(&p, "old").unwrap();
        let etag = save_atomic(&p, "new").unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), "new");
        assert_eq!(etag.size, 3);
        let entries: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(entries.len(), 1, "no leftover .tmp files");
    }
}
