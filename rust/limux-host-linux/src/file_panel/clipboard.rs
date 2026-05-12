use std::path::PathBuf;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipMode {
    Copy,
    Cut,
}

#[allow(dead_code)]
#[derive(Default, Debug)]
pub struct Clipboard {
    paths: Vec<PathBuf>,
    mode: Option<ClipMode>,
}

#[allow(dead_code)]
impl Clipboard {
    pub fn set(&mut self, paths: Vec<PathBuf>, mode: ClipMode) {
        self.paths = paths;
        self.mode = Some(mode);
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    pub fn mode(&self) -> Option<ClipMode> {
        self.mode
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn take_for_paste(&mut self) -> (Vec<PathBuf>, Option<ClipMode>) {
        let paths = self.paths.clone();
        let mode = self.mode;
        if matches!(mode, Some(ClipMode::Cut)) {
            self.paths.clear();
            self.mode = None;
        }
        (paths, mode)
    }

    pub fn clear(&mut self) {
        self.paths.clear();
        self.mode = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_clipboard_empty() {
        let c = Clipboard::default();
        assert!(c.is_empty());
    }

    #[test]
    fn copy_then_paste_keeps_state_until_explicit_clear() {
        let mut c = Clipboard::default();
        c.set(vec![PathBuf::from("/tmp/a")], ClipMode::Copy);
        assert!(!c.is_empty());
        assert_eq!(c.mode(), Some(ClipMode::Copy));
        let _ = c.take_for_paste();
        assert!(!c.is_empty(), "copy survives paste");
    }

    #[test]
    fn cut_then_paste_clears_state() {
        let mut c = Clipboard::default();
        c.set(vec![PathBuf::from("/tmp/a")], ClipMode::Cut);
        let _ = c.take_for_paste();
        assert!(c.is_empty(), "cut clears after paste");
    }

    #[test]
    fn explicit_clear_resets() {
        let mut c = Clipboard::default();
        c.set(vec![PathBuf::from("/tmp/a")], ClipMode::Copy);
        c.clear();
        assert!(c.is_empty());
        assert_eq!(c.mode(), None);
    }
}
