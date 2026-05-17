use std::cell::Cell;
use std::rc::Rc;

use gtk4 as gtk;
use gtk4::glib;
use gtk4::prelude::*;

pub const MINIMAP_NATURAL_WIDTH: i32 = 120;

pub fn apply_reservation(scrolled: &gtk::ScrolledWindow, visible: bool) {
    let margin = if visible { MINIMAP_NATURAL_WIDTH } else { 0 };
    scrolled.set_margin_end(margin);
}

#[derive(Clone)]
pub struct MinimapContainer {
    pub root: gtk::Overlay,
    #[allow(dead_code)]
    pub indicator: gtk::Box,
}

pub fn build(view: &sourceview5::View, map: &sourceview5::Map) -> MinimapContainer {
    map.remove_css_class("lyrux-editor-buffer");
    map.add_css_class("lyrux-minimap");

    let overlay = gtk::Overlay::new();
    overlay.add_css_class("lyrux-minimap-container");
    overlay.set_hexpand(false);
    overlay.set_vexpand(true);
    overlay.set_halign(gtk::Align::Fill);
    overlay.set_valign(gtk::Align::Fill);
    overlay.set_width_request(MINIMAP_NATURAL_WIDTH);
    overlay.set_child(Some(map));

    let indicator = gtk::Box::new(gtk::Orientation::Vertical, 0);
    indicator.add_css_class("lyrux-minimap-viewport");
    indicator.set_halign(gtk::Align::Fill);
    indicator.set_valign(gtk::Align::Start);
    indicator.set_hexpand(true);
    indicator.set_can_target(false);
    indicator.set_can_focus(false);
    indicator.set_visible(false);
    overlay.add_overlay(&indicator);

    wire_indicator(view, map, &indicator);

    MinimapContainer {
        root: overlay,
        indicator,
    }
}

fn wire_indicator(view: &sourceview5::View, map: &sourceview5::Map, indicator: &gtk::Box) {
    let pending = Rc::new(Cell::new(false));
    let view_weak = view.downgrade();
    let map_weak = map.downgrade();
    let indicator_weak = indicator.downgrade();
    let pending_for_fn = pending.clone();
    let refresh: Rc<dyn Fn()> = Rc::new(move || {
        if pending_for_fn.get() {
            return;
        }
        pending_for_fn.set(true);
        let view_inner = view_weak.clone();
        let map_inner = map_weak.clone();
        let indicator_inner = indicator_weak.clone();
        let pending_idle = pending_for_fn.clone();
        glib::idle_add_local_once(move || {
            pending_idle.set(false);
            let (Some(view), Some(map), Some(indicator)) = (
                view_inner.upgrade(),
                map_inner.upgrade(),
                indicator_inner.upgrade(),
            ) else {
                return;
            };
            update(&view, &map, &indicator);
        });
    });

    let scrolled_parent = view
        .parent()
        .and_then(|p| p.downcast::<gtk::ScrolledWindow>().ok());
    if let Some(scrolled) = scrolled_parent {
        let refresh_for_scroll = refresh.clone();
        scrolled.vadjustment().connect_value_changed(move |_| {
            refresh_for_scroll();
        });
        let refresh_for_changed = refresh.clone();
        scrolled.vadjustment().connect_changed(move |_| {
            refresh_for_changed();
        });
    }

    let refresh_for_buf = refresh.clone();
    view.buffer().connect_changed(move |_| {
        refresh_for_buf();
    });

    let refresh_for_map_height = refresh.clone();
    map.connect_height_request_notify(move |_| {
        refresh_for_map_height();
    });

    let refresh_for_map_realize = refresh.clone();
    map.connect_realize(move |_| {
        refresh_for_map_realize();
    });
    let refresh_for_map_show = refresh.clone();
    map.connect_show(move |_| {
        refresh_for_map_show();
    });

    let refresh_initial = refresh.clone();
    glib::idle_add_local_once(move || {
        refresh_initial();
    });
}

fn update(view: &sourceview5::View, map: &sourceview5::Map, indicator: &gtk::Box) {
    let map_height = map.height();
    if map_height <= 0 {
        indicator.set_visible(false);
        return;
    }

    let buffer = view.buffer();
    let start_iter = buffer.start_iter();
    let end_iter = buffer.end_iter();
    let (buf_top, _) = view.line_yrange(&start_iter);
    let (buf_bottom_start, buf_bottom_height) = view.line_yrange(&end_iter);
    let buf_total = (buf_bottom_start + buf_bottom_height - buf_top).max(1);

    let visible = view.visible_rect();
    let visible_top = (visible.y() - buf_top).max(0);
    let visible_height = visible.height().max(1);

    let ratio_top = (visible_top as f64 / buf_total as f64).clamp(0.0, 1.0);
    let ratio_height = (visible_height as f64 / buf_total as f64).clamp(0.0, 1.0);

    let margin_top = (ratio_top * map_height as f64).round() as i32;
    let mut height = (ratio_height * map_height as f64).round() as i32;
    if height < 8 {
        height = 8;
    }
    if margin_top + height > map_height {
        height = (map_height - margin_top).max(8);
    }

    indicator.set_margin_top(margin_top.max(0));
    indicator.set_height_request(height);
    indicator.set_visible(true);
}
