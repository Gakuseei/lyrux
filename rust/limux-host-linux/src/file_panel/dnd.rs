use gtk4 as gtk;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

use crate::file_panel::icons::icon_for_extension;
use crate::file_panel::model::Kind;
use crate::file_panel::row_object::RowObject;

const DRAG_ICON_SIZE: i32 = 24;

pub fn install_drag_source(
    widget: &impl IsA<gtk::Widget>,
    get_row: Rc<RefCell<Option<RowObject>>>,
) {
    let source = gtk::DragSource::new();
    source.set_actions(gdk::DragAction::COPY | gdk::DragAction::MOVE);
    let prepare_row = Rc::clone(&get_row);
    source.connect_prepare(move |_, _, _| {
        let row = prepare_row.borrow().clone()?;
        let file = gtk4::gio::File::for_path(row.path());
        let file_list = gdk::FileList::from_array(&[file]);
        let value = glib::Value::from(&file_list);
        Some(gdk::ContentProvider::for_value(&value))
    });
    let icon_row = Rc::clone(&get_row);
    let widget_clone = widget.as_ref().clone();
    source.connect_drag_begin(move |source, _drag| {
        let row = match icon_row.borrow().clone() {
            Some(r) => r,
            None => return,
        };
        let icon_name = if matches!(row.kind(), Kind::Dir) {
            "folder-symbolic"
        } else {
            let ext = row
                .name()
                .rsplit_once('.')
                .map(|(_, e)| e.to_string())
                .unwrap_or_default();
            icon_for_extension(&ext)
        };
        let display = WidgetExt::display(&widget_clone);
        let theme = gtk::IconTheme::for_display(&display);
        let paintable = theme.lookup_icon(
            icon_name,
            &[],
            DRAG_ICON_SIZE,
            widget_clone.scale_factor(),
            gtk::TextDirection::None,
            gtk::IconLookupFlags::empty(),
        );
        source.set_icon(Some(&paintable), 0, 0);
    });
    widget.add_controller(source);
}
