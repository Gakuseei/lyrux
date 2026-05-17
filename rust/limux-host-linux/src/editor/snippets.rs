#![allow(dead_code)]

pub const BUNDLED_SNIPPET_FILES: &[(&str, &str)] = &[
    ("c.snippets", include_str!("bundled_snippets/c.snippets")),
    (
        "css.snippets",
        include_str!("bundled_snippets/css.snippets"),
    ),
    ("go.snippets", include_str!("bundled_snippets/go.snippets")),
    (
        "html.snippets",
        include_str!("bundled_snippets/html.snippets"),
    ),
    ("js.snippets", include_str!("bundled_snippets/js.snippets")),
    (
        "json.snippets",
        include_str!("bundled_snippets/json.snippets"),
    ),
    (
        "licenses.snippets",
        include_str!("bundled_snippets/licenses.snippets"),
    ),
    (
        "markdown.snippets",
        include_str!("bundled_snippets/markdown.snippets"),
    ),
    (
        "python.snippets",
        include_str!("bundled_snippets/python.snippets"),
    ),
    (
        "rust.snippets",
        include_str!("bundled_snippets/rust.snippets"),
    ),
    ("sh.snippets", include_str!("bundled_snippets/sh.snippets")),
    (
        "shebang.snippets",
        include_str!("bundled_snippets/shebang.snippets"),
    ),
    (
        "toml.snippets",
        include_str!("bundled_snippets/toml.snippets"),
    ),
    (
        "typescript.snippets",
        include_str!("bundled_snippets/typescript.snippets"),
    ),
    (
        "xml.snippets",
        include_str!("bundled_snippets/xml.snippets"),
    ),
    (
        "yaml.snippets",
        include_str!("bundled_snippets/yaml.snippets"),
    ),
];

static REGISTERED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

pub fn register_bundled(manager: &sourceview5::SnippetManager) {
    REGISTERED.get_or_init(|| {
        let cache_dir = match dirs::cache_dir() {
            Some(d) => d.join("lyrux/sourceview-snippets"),
            None => return,
        };
        if std::fs::create_dir_all(&cache_dir).is_err() {
            return;
        }
        for (name, contents) in BUNDLED_SNIPPET_FILES {
            let path = cache_dir.join(name);
            let needs_write = match std::fs::read_to_string(&path) {
                Ok(existing) => existing != *contents,
                Err(_) => true,
            };
            if needs_write {
                let _ = std::fs::write(&path, contents);
            }
        }
        let cache_str = cache_dir.to_string_lossy().to_string();
        let mut paths: Vec<String> = manager
            .search_path()
            .iter()
            .map(|s| s.to_string())
            .collect();
        if !paths.contains(&cache_str) {
            paths.insert(0, cache_str);
            let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
            manager.set_search_path(&refs);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_file_is_xml() {
        for (name, body) in BUNDLED_SNIPPET_FILES {
            let trimmed = body.trim_start();
            assert!(
                trimmed.starts_with("<?xml") || trimmed.starts_with("<snippets"),
                "{name} does not start with XML declaration"
            );
        }
    }

    #[test]
    fn bundled_files_contain_at_least_one_snippet() {
        for (name, body) in BUNDLED_SNIPPET_FILES {
            if *name == "licenses.snippets" {
                continue;
            }
            assert!(
                body.contains("<snippet "),
                "{name} declares no <snippet> elements"
            );
        }
    }

    #[test]
    fn bundled_filenames_unique() {
        let mut names: Vec<&str> = BUNDLED_SNIPPET_FILES.iter().map(|(n, _)| *n).collect();
        names.sort();
        let len_before = names.len();
        names.dedup();
        assert_eq!(len_before, names.len(), "duplicate snippet filename");
    }
}
