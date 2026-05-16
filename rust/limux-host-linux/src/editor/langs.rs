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

pub fn register_bundled(manager: &sourceview5::LanguageManager) {
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
}

pub fn language_id_for_extension(ext: &str) -> Option<&'static str> {
    match ext {
        "rs" => Some("rust"),
        "ts" | "tsx" => Some("typescript"),
        "js" | "mjs" | "cjs" | "jsx" => Some("javascript"),
        "json" => Some("json"),
        "md" | "markdown" => Some("markdown"),
        "css" => Some("css"),
        "html" | "htm" => Some("html"),
        "py" => Some("python"),
        "toml" => Some("toml"),
        "yml" | "yaml" => Some("yaml"),
        "sh" | "bash" | "zsh" | "fish" => Some("sh"),
        "c" | "h" => Some("c"),
        "cpp" | "cc" | "cxx" | "hpp" => Some("cpp"),
        "go" => Some("go"),
        "lua" => Some("lua"),
        "rb" => Some("ruby"),
        "xml" | "svg" => Some("xml"),
        _ => None,
    }
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
        assert_eq!(ext_id("tsx"), Some("typescript"));
    }

    #[test]
    fn maps_json() {
        assert_eq!(ext_id("json"), Some("json"));
    }

    #[test]
    fn unknown_returns_none() {
        assert!(ext_id("zzzz").is_none());
    }
}
