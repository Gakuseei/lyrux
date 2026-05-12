use std::path::PathBuf;

// Phase-2 helpers for persisting expanded-paths to config. Wired up when
// session restore lands.

#[allow(dead_code)]
pub fn paths_to_strings(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect()
}

#[allow(dead_code)]
pub fn strings_to_paths(s: &[String]) -> Vec<PathBuf> {
    s.iter().map(PathBuf::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_paths() {
        let p = vec![PathBuf::from("/tmp/a"), PathBuf::from("/tmp/b")];
        let s = paths_to_strings(&p);
        assert_eq!(strings_to_paths(&s), p);
    }
}
