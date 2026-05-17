use std::cell::RefCell;
use std::path::PathBuf;

pub const MAX_CLOSED_TABS: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosedTab {
    pub path: PathBuf,
    pub cursor_offset: i32,
}

thread_local! {
    static CLOSED_TABS: RefCell<Vec<ClosedTab>> = const { RefCell::new(Vec::new()) };
}

pub fn push(tab: ClosedTab) {
    CLOSED_TABS.with(|slot| {
        let mut stack = slot.borrow_mut();
        stack.retain(|existing| existing.path != tab.path);
        stack.push(tab);
        if stack.len() > MAX_CLOSED_TABS {
            let overflow = stack.len() - MAX_CLOSED_TABS;
            stack.drain(0..overflow);
        }
    });
}

pub fn pop() -> Option<ClosedTab> {
    CLOSED_TABS.with(|slot| slot.borrow_mut().pop())
}

#[cfg(test)]
pub fn clear() {
    CLOSED_TABS.with(|slot| slot.borrow_mut().clear());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, offset: i32) -> ClosedTab {
        ClosedTab {
            path: PathBuf::from(path),
            cursor_offset: offset,
        }
    }

    #[test]
    fn pop_returns_lifo_order() {
        clear();
        push(entry("/a/one.rs", 0));
        push(entry("/a/two.rs", 5));
        assert_eq!(pop().map(|t| t.path), Some(PathBuf::from("/a/two.rs")));
        assert_eq!(pop().map(|t| t.path), Some(PathBuf::from("/a/one.rs")));
        assert_eq!(pop(), None);
    }

    #[test]
    fn push_dedupes_existing_path() {
        clear();
        push(entry("/a/one.rs", 0));
        push(entry("/a/two.rs", 0));
        push(entry("/a/one.rs", 10));
        let first = pop().expect("entry");
        assert_eq!(first.path, PathBuf::from("/a/one.rs"));
        assert_eq!(first.cursor_offset, 10);
        let second = pop().expect("entry");
        assert_eq!(second.path, PathBuf::from("/a/two.rs"));
    }

    #[test]
    fn push_caps_at_max() {
        clear();
        for i in 0..(MAX_CLOSED_TABS + 5) {
            push(entry(&format!("/a/{i}.rs"), 0));
        }
        let mut count = 0;
        while pop().is_some() {
            count += 1;
        }
        assert_eq!(count, MAX_CLOSED_TABS);
    }
}
