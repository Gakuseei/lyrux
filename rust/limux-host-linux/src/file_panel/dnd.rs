use gtk4 as gtk;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;

use crate::file_panel::row_object::RowObject;

#[allow(dead_code)]
pub fn install_drag_source(
    widget: &impl IsA<gtk::Widget>,
    get_row: impl Fn() -> Option<RowObject> + 'static,
) {
    let source = gtk::DragSource::new();
    source.set_actions(gdk::DragAction::COPY | gdk::DragAction::MOVE);
    source.connect_prepare(move |_, _, _| {
        let row = get_row()?;
        let file = gtk4::gio::File::for_path(row.path());
        let value = glib::Value::from(&file);
        Some(gdk::ContentProvider::for_value(&value))
    });
    widget.add_controller(source);
}
