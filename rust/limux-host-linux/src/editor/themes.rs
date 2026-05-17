#![allow(dead_code)]
use crate::editor::strings;

const SCHEMES: &[(&str, &str, &str)] = &[
    (
        "lyrux-grey",
        strings::THEME_LYRUX_GREY,
        include_str!("bundled_styles/lyrux-grey.xml"),
    ),
    (
        "lyrux-dark",
        strings::THEME_LYRUX_DARK,
        include_str!("bundled_styles/lyrux-dark.xml"),
    ),
    (
        "lyrux-light",
        strings::THEME_LYRUX_LIGHT,
        include_str!("bundled_styles/lyrux-light.xml"),
    ),
    (
        "catppuccin-latte",
        strings::THEME_CATPPUCCIN_LATTE,
        include_str!("bundled_styles/catppuccin-latte.xml"),
    ),
    (
        "catppuccin-frappe",
        strings::THEME_CATPPUCCIN_FRAPPE,
        include_str!("bundled_styles/catppuccin-frappe.xml"),
    ),
    (
        "catppuccin-macchiato",
        strings::THEME_CATPPUCCIN_MACCHIATO,
        include_str!("bundled_styles/catppuccin-macchiato.xml"),
    ),
    (
        "catppuccin-mocha",
        strings::THEME_CATPPUCCIN_MOCHA,
        include_str!("bundled_styles/catppuccin-mocha.xml"),
    ),
    (
        "tokyo-night",
        strings::THEME_TOKYO_NIGHT,
        include_str!("bundled_styles/tokyo-night.xml"),
    ),
    (
        "tokyo-night-storm",
        strings::THEME_TOKYO_NIGHT_STORM,
        include_str!("bundled_styles/tokyo-night-storm.xml"),
    ),
    (
        "one-dark",
        strings::THEME_ONE_DARK,
        include_str!("bundled_styles/one-dark.xml"),
    ),
    (
        "one-light",
        strings::THEME_ONE_LIGHT,
        include_str!("bundled_styles/one-light.xml"),
    ),
];

static REGISTERED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

pub fn register_all(manager: &sourceview5::StyleSchemeManager) {
    REGISTERED.get_or_init(|| {
        let cache_dir = match dirs::cache_dir() {
            Some(d) => d.join("lyrux/sourceview-styles"),
            None => return,
        };
        if std::fs::create_dir_all(&cache_dir).is_err() {
            return;
        }
        for (id, _label, xml) in SCHEMES {
            let path = cache_dir.join(format!("{id}.xml"));
            let needs_write = match std::fs::read_to_string(&path) {
                Ok(existing) => existing.as_str() != *xml,
                Err(_) => true,
            };
            if needs_write {
                let _ = std::fs::write(&path, xml);
            }
        }
        let mut paths: Vec<String> = manager
            .search_path()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let cache_str = cache_dir.to_string_lossy().to_string();
        if !paths.contains(&cache_str) {
            paths.push(cache_str);
        }
        if let Some(config_dir) = dirs::config_dir() {
            let user_themes = config_dir.join("lyrux/themes");
            std::fs::create_dir_all(&user_themes).ok();
            let user_str = user_themes.to_string_lossy().to_string();
            if !paths.contains(&user_str) {
                paths.insert(0, user_str);
            }
        }
        let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
        manager.set_search_path(&refs);
    });
}

pub fn available() -> &'static [(&'static str, &'static str, &'static str)] {
    SCHEMES
}

pub fn available_dynamic() -> Vec<(String, String)> {
    let manager = sourceview5::StyleSchemeManager::default();
    register_all(&manager);
    let mut entries: Vec<(String, String)> = manager
        .scheme_ids()
        .iter()
        .map(|id| {
            let id_str = id.to_string();
            let label = display_label(&id_str);
            (id_str, label)
        })
        .collect();
    entries.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
    entries
}

pub fn label_for(id: &str) -> Option<&'static str> {
    SCHEMES
        .iter()
        .find(|(i, _, _)| *i == id)
        .map(|(_, l, _)| *l)
}

pub fn display_label(id: &str) -> String {
    if let Some(bundled) = label_for(id) {
        return bundled.to_string();
    }
    let manager = sourceview5::StyleSchemeManager::default();
    if let Some(scheme) = manager.scheme(id) {
        let name = scheme.name().to_string();
        if !name.is_empty() {
            return name;
        }
    }
    id.to_string()
}

pub fn default_id() -> &'static str {
    "lyrux-grey"
}

pub fn default_light_id() -> &'static str {
    "lyrux-light"
}

pub fn default_dark_id() -> &'static str {
    "lyrux-grey"
}
