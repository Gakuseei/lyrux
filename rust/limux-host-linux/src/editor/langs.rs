#![allow(dead_code)]

pub const BUNDLED_LANG_FILES: &[(&str, &str)] = &[
    ("def.lang", include_str!("bundled_langs/def.lang")),
    ("rust.lang", include_str!("bundled_langs/rust.lang")),
    (
        "typescript.lang",
        include_str!("bundled_langs/typescript.lang"),
    ),
    (
        "typescript-jsx.lang",
        include_str!("bundled_langs/typescript-jsx.lang"),
    ),
    (
        "typescript-type-expressions.lang",
        include_str!("bundled_langs/typescript-type-expressions.lang"),
    ),
    (
        "typescript-type-generics.lang",
        include_str!("bundled_langs/typescript-type-generics.lang"),
    ),
    (
        "typescript-type-literals.lang",
        include_str!("bundled_langs/typescript-type-literals.lang"),
    ),
    (
        "javascript.lang",
        include_str!("bundled_langs/javascript.lang"),
    ),
    (
        "javascript-expressions.lang",
        include_str!("bundled_langs/javascript-expressions.lang"),
    ),
    (
        "javascript-functions-classes.lang",
        include_str!("bundled_langs/javascript-functions-classes.lang"),
    ),
    (
        "javascript-literals.lang",
        include_str!("bundled_langs/javascript-literals.lang"),
    ),
    (
        "javascript-modules.lang",
        include_str!("bundled_langs/javascript-modules.lang"),
    ),
    (
        "javascript-statements.lang",
        include_str!("bundled_langs/javascript-statements.lang"),
    ),
    (
        "javascript-values.lang",
        include_str!("bundled_langs/javascript-values.lang"),
    ),
    ("json.lang", include_str!("bundled_langs/json.lang")),
    ("markdown.lang", include_str!("bundled_langs/markdown.lang")),
    ("css.lang", include_str!("bundled_langs/css.lang")),
    ("html.lang", include_str!("bundled_langs/html.lang")),
    ("python.lang", include_str!("bundled_langs/python.lang")),
    ("python3.lang", include_str!("bundled_langs/python3.lang")),
    ("toml.lang", include_str!("bundled_langs/toml.lang")),
    ("yaml.lang", include_str!("bundled_langs/yaml.lang")),
    ("sh.lang", include_str!("bundled_langs/sh.lang")),
    ("c.lang", include_str!("bundled_langs/c.lang")),
    ("cpp.lang", include_str!("bundled_langs/cpp.lang")),
    ("chdr.lang", include_str!("bundled_langs/chdr.lang")),
    ("go.lang", include_str!("bundled_langs/go.lang")),
    ("lua.lang", include_str!("bundled_langs/lua.lang")),
    ("ruby.lang", include_str!("bundled_langs/ruby.lang")),
    ("xml.lang", include_str!("bundled_langs/xml.lang")),
    ("dtd.lang", include_str!("bundled_langs/dtd.lang")),
];

static REGISTERED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

pub fn register_bundled(manager: &sourceview5::LanguageManager) {
    REGISTERED.get_or_init(|| {
        let cache_dir = match dirs::cache_dir() {
            Some(d) => d.join("lyrux/sourceview-langs"),
            None => return,
        };
        if std::fs::create_dir_all(&cache_dir).is_err() {
            return;
        }
        for (name, contents) in BUNDLED_LANG_FILES {
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

pub fn language_id_for_extension(ext: &str) -> Option<&'static str> {
    let id = match ext {
        "rs" => "rust",
        "ts" => "typescript",
        "tsx" => "typescript-jsx",
        "js" | "mjs" | "cjs" | "jsx" => "js",
        "json" => "json",
        "md" | "markdown" => "markdown",
        "css" => "css",
        "html" | "htm" => "html",
        "py" | "pyi" | "py3" => "python3",
        "toml" => "toml",
        "yml" | "yaml" => "yaml",
        "sh" | "bash" | "zsh" | "fish" => "sh",
        "c" => "c",
        "h" => "chdr",
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => "cpp",
        "go" => "go",
        "lua" => "lua",
        "rb" => "ruby",
        "xml" | "svg" => "xml",
        "dtd" => "dtd",
        _ => return None,
    };
    Some(id)
}

pub fn language_for_path(path: &std::path::Path) -> Option<sourceview5::Language> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())?
        .to_ascii_lowercase();
    let id = language_id_for_extension(ext.as_str())?;
    register_bundled(&sourceview5::LanguageManager::default());
    sourceview5::LanguageManager::default().language(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ext_id(ext: &str) -> Option<&'static str> {
        language_id_for_extension(ext)
    }

    #[test]
    fn maps_rust() {
        assert_eq!(ext_id("rs"), Some("rust"));
    }

    #[test]
    fn maps_typescript() {
        assert_eq!(ext_id("ts"), Some("typescript"));
        assert_eq!(ext_id("tsx"), Some("typescript-jsx"));
    }

    #[test]
    fn maps_json() {
        assert_eq!(ext_id("json"), Some("json"));
    }

    #[test]
    fn maps_javascript_family() {
        assert_eq!(ext_id("js"), Some("js"));
        assert_eq!(ext_id("jsx"), Some("js"));
        assert_eq!(ext_id("mjs"), Some("js"));
        assert_eq!(ext_id("cjs"), Some("js"));
    }

    #[test]
    fn maps_python_family() {
        assert_eq!(ext_id("py"), Some("python3"));
        assert_eq!(ext_id("pyi"), Some("python3"));
    }

    #[test]
    fn maps_c_header_separately() {
        assert_eq!(ext_id("c"), Some("c"));
        assert_eq!(ext_id("h"), Some("chdr"));
    }

    #[test]
    fn unknown_returns_none() {
        assert!(ext_id("zzzz").is_none());
    }

    fn collect_bundled_ids() -> std::collections::HashSet<&'static str> {
        let mut ids = std::collections::HashSet::new();
        for (_name, body) in BUNDLED_LANG_FILES {
            let mut rest = *body;
            while let Some(idx) = rest.find("<language") {
                rest = &rest[idx + "<language".len()..];
                let close = match rest.find('>') {
                    Some(c) => c,
                    None => break,
                };
                let attrs = &rest[..close];
                if let Some(id_idx) = attrs.find("id=\"") {
                    let after = &attrs[id_idx + 4..];
                    if let Some(end) = after.find('"') {
                        ids.insert(&after[..end]);
                    }
                }
                rest = &rest[close + 1..];
            }
        }
        ids
    }

    #[test]
    fn every_mapped_id_exists_in_bundled_langs() {
        let bundled = collect_bundled_ids();
        let exts = [
            "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "json", "md", "css", "html", "htm", "py",
            "pyi", "py3", "toml", "yml", "yaml", "sh", "bash", "zsh", "fish", "c", "h", "cpp",
            "cc", "cxx", "hpp", "hxx", "go", "lua", "rb", "xml", "svg", "dtd",
        ];
        for ext in exts {
            let id = ext_id(ext).unwrap_or_else(|| panic!("no mapping for {ext}"));
            assert!(
                bundled.contains(id),
                "extension {ext} maps to {id} but no bundled .lang declares that id"
            );
        }
    }
}
