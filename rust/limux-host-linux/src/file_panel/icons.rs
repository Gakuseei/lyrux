pub const GENERIC_FILE_ICON: &str = "text-x-generic-symbolic";

pub fn icon_for_extension(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "cpp" | "cc" | "cxx" | "c" | "h"
        | "hpp" | "java" | "kt" | "swift" | "rb" | "php" | "lua" | "zig" | "sh" | "bash"
        | "fish" | "zsh" => "text-x-script-symbolic",
        "html" | "htm" | "xhtml" => "text-html-symbolic",
        "css" | "scss" | "sass" | "less" => "text-x-generic-symbolic",
        "json" | "jsonc" => "text-x-generic-symbolic",
        "xml" | "svg" => "text-x-generic-symbolic",
        "yml" | "yaml" => "text-x-generic-symbolic",
        "toml" | "ini" | "cfg" | "conf" => "text-x-generic-symbolic",
        "md" | "markdown" | "rst" | "txt" => "text-x-generic-symbolic",
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tiff" | "ico" | "avif" => {
            "image-x-generic-symbolic"
        }
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "tgz" | "zst" => {
            "package-x-generic-symbolic"
        }
        "pdf" => "application-pdf-symbolic",
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" => "audio-x-generic-symbolic",
        "mp4" | "mkv" | "webm" | "mov" | "avi" | "wmv" | "flv" => "video-x-generic-symbolic",
        _ => "text-x-generic-symbolic",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_files_get_script_icon() {
        assert_eq!(icon_for_extension("rs"), "text-x-script-symbolic");
    }

    #[test]
    fn images_get_image_icon() {
        assert_eq!(icon_for_extension("png"), "image-x-generic-symbolic");
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(icon_for_extension("PNG"), "image-x-generic-symbolic");
    }

    #[test]
    fn unknown_falls_back_to_generic() {
        assert_eq!(icon_for_extension("xyz"), "text-x-generic-symbolic");
    }

    #[test]
    fn html_uses_symbolic_icon() {
        assert_eq!(icon_for_extension("html"), "text-html-symbolic");
    }

    #[test]
    fn json_falls_back_to_generic_symbolic() {
        assert_eq!(icon_for_extension("json"), "text-x-generic-symbolic");
    }

    #[test]
    fn css_uses_symbolic_icon() {
        assert_eq!(icon_for_extension("css"), "text-x-generic-symbolic");
    }

    #[test]
    fn generic_constant_matches_fallback() {
        assert_eq!(GENERIC_FILE_ICON, "text-x-generic-symbolic");
        assert_eq!(icon_for_extension("xyz"), GENERIC_FILE_ICON);
    }
}
