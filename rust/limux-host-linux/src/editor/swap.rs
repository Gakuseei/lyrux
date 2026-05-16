use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn swap_dir(workspace_id: &str, pane_id: &str) -> Option<PathBuf> {
    let base = dirs::state_dir()?;
    Some(
        base.join("lyrux/swap")
            .join(sanitize(workspace_id))
            .join(sanitize(pane_id)),
    )
}

pub fn write(workspace_id: &str, pane_id: &str, contents: &str) -> io::Result<PathBuf> {
    let dir = swap_dir(workspace_id, pane_id).ok_or_else(|| io::Error::other("no state dir"))?;
    fs::create_dir_all(&dir)?;
    let file = dir.join(format!("{}.swap", uuid::Uuid::new_v4()));
    fs::write(&file, contents)?;
    Ok(file)
}

pub fn read(path: &Path) -> io::Result<String> {
    fs::read_to_string(path)
}

pub fn discard(path: &Path) -> io::Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn sanitize(id: &str) -> String {
    let cleaned: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "_".to_string()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let p = write("test-ws", "test-pane", "hello").expect("write");
        assert_eq!(read(&p).expect("read"), "hello");
        discard(&p).expect("discard");
        assert!(!p.exists());
    }
}
