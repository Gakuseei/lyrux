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
    pub fit_to_window: Rc<Cell<bool>>,
    pub last_viewport: Rc<Cell<(i32, i32)>>,
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
        fit_to_window: Rc::new(Cell::new(true)),
        last_viewport: Rc::new(Cell::new((0, 0))),
        root: scrolled.clone().upcast(),
    };
    install_zoom_controller(&state);
    install_fit_handler(&state);
    state.apply_zoom();
    Some(state)
}

pub fn compute_fit_zoom(
    natural_w: i32,
    natural_h: i32,
    viewport_w: i32,
    viewport_h: i32,
    min_zoom: f64,
    max_zoom: f64,
) -> Option<f64> {
    if viewport_w <= 0 || viewport_h <= 0 || natural_w <= 0 || natural_h <= 0 {
        return None;
    }
    let fit = (viewport_w as f64 / natural_w as f64)
        .min(viewport_h as f64 / natural_h as f64)
        .min(1.0);
    Some(fit.clamp(min_zoom, max_zoom))
}

impl ImageViewerTabState {
    pub fn apply_zoom(&self) {
        let (w, h) = self.natural.get();
        if self.fit_to_window.get() {
            let vp_w = self.scrolled.width();
            let vp_h = self.scrolled.height();
            if let Some(z) = compute_fit_zoom(w, h, vp_w, vp_h, MIN_ZOOM, MAX_ZOOM) {
                self.zoom.set(z);
            }
        }
        let z = self.zoom.get().clamp(MIN_ZOOM, MAX_ZOOM);
        self.zoom.set(z);
        let target_w = (w as f64 * z) as i32;
        let target_h = (h as f64 * z) as i32;
        if self.picture.width_request() != target_w || self.picture.height_request() != target_h {
            self.picture.set_size_request(target_w, target_h);
        }
    }
}

fn install_fit_handler(state: &ImageViewerTabState) {
    let s = state.clone();
    let _tick_id = state.scrolled.add_tick_callback(move |w, _| {
        let cur = (w.width(), w.height());
        if cur != s.last_viewport.get() {
            s.last_viewport.set(cur);
            if s.fit_to_window.get() {
                s.apply_zoom();
            }
        }
        gtk4::glib::ControlFlow::Continue
    });
    let s = state.clone();
    state.scrolled.connect_map(move |_| {
        let s = s.clone();
        gtk4::glib::idle_add_local_once(move || s.apply_zoom());
    });
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
            s.fit_to_window.set(false);
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
                gtk4::gdk::Key::_0 => {
                    s.fit_to_window.set(true);
                    s.apply_zoom();
                    gtk4::glib::Propagation::Stop
                }
                gtk4::gdk::Key::_1 => {
                    s.fit_to_window.set(false);
                    s.zoom.set(1.0);
                    s.apply_zoom();
                    gtk4::glib::Propagation::Stop
                }
                gtk4::gdk::Key::plus | gtk4::gdk::Key::equal | gtk4::gdk::Key::KP_Add => {
                    s.fit_to_window.set(false);
                    s.zoom.set(s.zoom.get() * STEP);
                    s.apply_zoom();
                    gtk4::glib::Propagation::Stop
                }
                gtk4::gdk::Key::minus | gtk4::gdk::Key::KP_Subtract => {
                    s.fit_to_window.set(false);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_shrinks_oversized() {
        let z = compute_fit_zoom(4000, 3000, 800, 600, MIN_ZOOM, MAX_ZOOM).unwrap();
        assert!((z - 0.2).abs() < 1e-9, "expected ~0.2, got {z}");
    }

    #[test]
    fn fit_does_not_upscale_small() {
        let z = compute_fit_zoom(100, 100, 800, 600, MIN_ZOOM, MAX_ZOOM).unwrap();
        assert_eq!(z, 1.0);
    }

    #[test]
    fn fit_clamps_to_min_zoom() {
        let z = compute_fit_zoom(50_000, 50_000, 100, 100, MIN_ZOOM, MAX_ZOOM).unwrap();
        assert!(z >= MIN_ZOOM);
        assert!((z - MIN_ZOOM).abs() < 1e-9, "expected MIN_ZOOM, got {z}");
    }

    #[test]
    fn fit_zero_viewport_is_skip() {
        assert!(compute_fit_zoom(4000, 3000, 0, 0, MIN_ZOOM, MAX_ZOOM).is_none());
        assert!(compute_fit_zoom(4000, 3000, 800, 0, MIN_ZOOM, MAX_ZOOM).is_none());
        assert!(compute_fit_zoom(4000, 3000, 0, 600, MIN_ZOOM, MAX_ZOOM).is_none());
    }

    #[test]
    fn fit_zero_natural_is_skip() {
        assert!(compute_fit_zoom(0, 0, 800, 600, MIN_ZOOM, MAX_ZOOM).is_none());
    }

    #[test]
    fn fit_picks_smaller_axis_ratio() {
        let z = compute_fit_zoom(2000, 1000, 800, 600, MIN_ZOOM, MAX_ZOOM).unwrap();
        assert!(
            (z - 0.4).abs() < 1e-9,
            "expected 0.4 (width-bound), got {z}"
        );
    }
}
