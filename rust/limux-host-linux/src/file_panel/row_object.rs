use std::cell::RefCell;
use std::path::PathBuf;

use gtk4::glib;
use gtk4::glib::Properties;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;

use crate::file_panel::model::{GitStatus, Kind, Row};

mod imp {
    use super::*;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::RowObject)]
    pub struct RowObject {
        #[property(get, set)]
        pub path_str: RefCell<String>,
        #[property(get, set)]
        pub depth: std::cell::Cell<u32>,
        #[property(get, set)]
        pub kind_id: std::cell::Cell<i32>,
        #[property(get, set)]
        pub expanded: std::cell::Cell<bool>,
        #[property(get, set)]
        pub git_id: std::cell::Cell<i32>,
        #[property(get, set)]
        pub name: RefCell<String>,
        #[property(get, set)]
        pub ignored: std::cell::Cell<bool>,
        #[property(get, set)]
        pub size: std::cell::Cell<u64>,
        #[property(get, set)]
        pub mtime: std::cell::Cell<i64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RowObject {
        const NAME: &'static str = "LimuxFpRowObject";
        type Type = super::RowObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for RowObject {}
}

glib::wrapper! {
    pub struct RowObject(ObjectSubclass<imp::RowObject>);
}

impl RowObject {
    pub fn from_row(row: &Row) -> Self {
        let obj: Self = glib::Object::new();
        obj.set_path_str(row.path.to_string_lossy().to_string());
        obj.set_depth(row.depth);
        obj.set_kind_id(kind_to_id(row.kind));
        obj.set_expanded(row.expanded);
        obj.set_git_id(git_to_id(row.git_status));
        obj.set_name(
            row.path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
        );
        obj.set_ignored(row.ignored);
        obj.set_size(row.size);
        obj.set_mtime(row.mtime);
        obj
    }

    pub fn path(&self) -> PathBuf {
        PathBuf::from(self.path_str())
    }

    pub fn kind(&self) -> Kind {
        match self.kind_id() {
            0 => Kind::Dir,
            1 => Kind::File,
            _ => Kind::Symlink,
        }
    }

    pub fn git_status(&self) -> GitStatus {
        id_to_git(self.git_id())
    }

    pub fn matches_row(&self, row: &Row) -> bool {
        self.path_str() == row.path.to_string_lossy()
            && self.depth() == row.depth
            && self.kind_id() == kind_to_id(row.kind)
            && self.expanded() == row.expanded
            && self.git_id() == git_to_id(row.git_status)
            && self.ignored() == row.ignored
            && self.size() == row.size
            && self.mtime() == row.mtime
    }
}

fn kind_to_id(k: Kind) -> i32 {
    match k {
        Kind::Dir => 0,
        Kind::File => 1,
        Kind::Symlink => 2,
    }
}

fn git_to_id(g: GitStatus) -> i32 {
    match g {
        GitStatus::Clean => 0,
        GitStatus::Modified => 1,
        GitStatus::Added => 2,
        GitStatus::Deleted => 3,
        GitStatus::Untracked => 4,
        GitStatus::Conflict => 5,
        GitStatus::Ignored => 6,
    }
}

fn id_to_git(id: i32) -> GitStatus {
    match id {
        1 => GitStatus::Modified,
        2 => GitStatus::Added,
        3 => GitStatus::Deleted,
        4 => GitStatus::Untracked,
        5 => GitStatus::Conflict,
        6 => GitStatus::Ignored,
        _ => GitStatus::Clean,
    }
}
