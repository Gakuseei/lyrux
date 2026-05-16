use std::path::Path;

use crate::editor::keymap::SaveTransform;
use crate::editor::view::ViewConfig;

pub fn apply_view_overrides(path: &Path, cfg: &mut ViewConfig) {
    let Some(props) = load_props(path) else {
        return;
    };
    if let Some(val) = props.get_raw_for_key("indent_style").into_option() {
        cfg.insert_spaces = val.eq_ignore_ascii_case("space");
    }
    if let Some(val) = props.get_raw_for_key("indent_size").into_option() {
        if let Ok(n) = val.parse::<u32>() {
            cfg.tab_width = n.clamp(1, 16);
        }
    }
    if let Some(val) = props.get_raw_for_key("tab_width").into_option() {
        if let Ok(n) = val.parse::<u32>() {
            cfg.tab_width = n.clamp(1, 16);
        }
    }
}

pub fn apply_save_overrides(path: &Path, transform: &mut SaveTransform) {
    let Some(props) = load_props(path) else {
        return;
    };
    if let Some(val) = props
        .get_raw_for_key("trim_trailing_whitespace")
        .into_option()
    {
        transform.strip_trailing_whitespace = val.eq_ignore_ascii_case("true");
    }
    if let Some(val) = props.get_raw_for_key("insert_final_newline").into_option() {
        transform.ensure_final_newline = val.eq_ignore_ascii_case("true");
    }
}

fn load_props(path: &Path) -> Option<ec4rs::Properties> {
    let mut props = ec4rs::properties_of(path).ok()?;
    props.use_fallbacks();
    Some(props)
}
