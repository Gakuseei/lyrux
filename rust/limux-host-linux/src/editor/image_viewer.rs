use gtk4::prelude::*;
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

const MIN_ZOOM: f64 = 0.05;
const MAX_ZOOM: f64 = 16.0;
const STEP: f64 = 1.1;

#[derive(Clone)]
pub struct ImageViewerTabState {
    pub path: PathBuf,
    pub picture: gtk4::Picture,
    pub scrolled: gtk4::ScrolledWindow,
    pub zoom: Rc<Cell<f64>>,
    pub natural: Rc<Cell<(i32, i32)>>,
    pub root: gtk4::Widget,
}

pub fn build(path: PathBuf) -> Option<ImageViewerTabState> {
    let texture = gtk4::gdk::Texture::from_filename(&path).ok()?;
    let picture = gtk4::Picture::for_paintable(&texture);
    picture.set_can_shrink(false);
    let natural = (texture.width(), texture.height());

    let scrolled = gtk4::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&picture)
        .build();
    scrolled.set_focusable(true);

    let state = ImageViewerTabState {
        path,
        picture: picture.clone(),
        scrolled: scrolled.clone(),
        zoom: Rc::new(Cell::new(1.0)),
        natural: Rc::new(Cell::new(natural)),
        root: scrolled.clone().upcast(),
    };
    install_zoom_controller(&state);
    state.apply_zoom();
    Some(state)
}

impl ImageViewerTabState {
    pub fn apply_zoom(&self) {
        let (w, h) = self.natural.get();
        let z = self.zoom.get().clamp(MIN_ZOOM, MAX_ZOOM);
        self.zoom.set(z);
        self.picture
            .set_size_request((w as f64 * z) as i32, (h as f64 * z) as i32);
    }
}

fn install_zoom_controller(state: &ImageViewerTabState) {
    let scroll_ctrl = gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
    {
        let s = state.clone();
        scroll_ctrl.connect_scroll(move |c, _dx, dy| {
            if let Some(ev) = c.current_event() {
                if !ev
                    .modifier_state()
                    .contains(gtk4::gdk::ModifierType::CONTROL_MASK)
                {
                    return gtk4::glib::Propagation::Proceed;
                }
            } else {
                return gtk4::glib::Propagation::Proceed;
            }
            let z = s.zoom.get();
            if dy < 0.0 {
                s.zoom.set(z * STEP);
            } else if dy > 0.0 {
                s.zoom.set(z / STEP);
            }
            s.apply_zoom();
            gtk4::glib::Propagation::Stop
        });
    }
    state.scrolled.add_controller(scroll_ctrl);

    let key_ctrl = gtk4::EventControllerKey::new();
    {
        let s = state.clone();
        key_ctrl.connect_key_pressed(move |_, key, _, mods| {
            if !mods.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                return gtk4::glib::Propagation::Proceed;
            }
            match key {
                gtk4::gdk::Key::_0 | gtk4::gdk::Key::_1 => {
                    s.zoom.set(1.0);
                    s.apply_zoom();
                    gtk4::glib::Propagation::Stop
                }
                gtk4::gdk::Key::plus | gtk4::gdk::Key::equal | gtk4::gdk::Key::KP_Add => {
                    s.zoom.set(s.zoom.get() * STEP);
                    s.apply_zoom();
                    gtk4::glib::Propagation::Stop
                }
                gtk4::gdk::Key::minus | gtk4::gdk::Key::KP_Subtract => {
                    s.zoom.set(s.zoom.get() / STEP);
                    s.apply_zoom();
                    gtk4::glib::Propagation::Stop
                }
                _ => gtk4::glib::Propagation::Proceed,
            }
        });
    }
    state.scrolled.add_controller(key_ctrl);
}
