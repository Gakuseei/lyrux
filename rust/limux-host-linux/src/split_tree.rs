use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;

use crate::layout_state::{self, LayoutNodeState, PaneState, SplitOrientation, SplitState};
use crate::pane;
use crate::window::{
    apply_split_ratio_after_layout, attach_split_position_persistence, update_split_ratio_state,
    State,
};

pub(crate) enum SplitNode {
    Leaf {
        pane_widget: gtk::Widget,
    },
    Split {
        orientation: gtk::Orientation,
        ratio: Rc<RefCell<f64>>,
        left: Box<SplitNode>,
        right: Box<SplitNode>,
    },
}

impl SplitNode {
    pub(crate) fn is_leaf(&self) -> bool {
        matches!(self, SplitNode::Leaf { .. })
    }

    pub(crate) fn replace(&mut self, target: &gtk::Widget, replacement: SplitNode) -> bool {
        match self {
            SplitNode::Leaf { pane_widget } => {
                if pane_widget == target {
                    *self = replacement;
                    true
                } else {
                    false
                }
            }
            SplitNode::Split { left, right, .. } => {
                if left.contains_pane(target) {
                    left.replace(target, replacement)
                } else {
                    right.replace(target, replacement)
                }
            }
        }
    }

    fn contains_pane(&self, target: &gtk::Widget) -> bool {
        match self {
            SplitNode::Leaf { pane_widget } => pane_widget == target,
            SplitNode::Split { left, right, .. } => {
                left.contains_pane(target) || right.contains_pane(target)
            }
        }
    }

    pub(crate) fn remove(&mut self, target: &gtk::Widget) -> bool {
        match self {
            SplitNode::Leaf { .. } => false,
            SplitNode::Split { left, right, .. } => {
                if matches!(left.as_ref(), SplitNode::Leaf { pane_widget } if pane_widget == target)
                {
                    *self = std::mem::replace(
                        right.as_mut(),
                        SplitNode::Leaf {
                            pane_widget: target.clone(),
                        },
                    );
                    return true;
                }
                if matches!(right.as_ref(), SplitNode::Leaf { pane_widget } if pane_widget == target)
                {
                    *self = std::mem::replace(
                        left.as_mut(),
                        SplitNode::Leaf {
                            pane_widget: target.clone(),
                        },
                    );
                    return true;
                }
                left.remove(target) || right.remove(target)
            }
        }
    }

    pub(crate) fn snapshot(
        &self,
        working_directory: Option<&str>,
        workspace_id: &str,
    ) -> LayoutNodeState {
        match self {
            SplitNode::Leaf { pane_widget } => pane::snapshot_pane_state(pane_widget, workspace_id)
                .map(LayoutNodeState::Pane)
                .unwrap_or_else(|| LayoutNodeState::Pane(PaneState::fallback(working_directory))),
            SplitNode::Split {
                orientation,
                ratio,
                left,
                right,
            } => LayoutNodeState::Split(SplitState {
                orientation: if *orientation == gtk::Orientation::Horizontal {
                    SplitOrientation::Horizontal
                } else {
                    SplitOrientation::Vertical
                },
                ratio: *ratio.borrow(),
                start: Box::new(left.snapshot(working_directory, workspace_id)),
                end: Box::new(right.snapshot(working_directory, workspace_id)),
            }),
        }
    }
}

pub(crate) struct SplitTreeContainer {
    tree: RefCell<SplitNode>,
    bin: gtk::Box,
    rebuild_source: RefCell<Option<glib::SourceId>>,
    last_focused: RefCell<Option<gtk::Widget>>,
    state: State,
}

impl SplitTreeContainer {
    pub(crate) fn new(state: &State, initial_pane: gtk::Widget) -> Rc<Self> {
        let bin = gtk::Box::new(gtk::Orientation::Vertical, 0);
        bin.set_hexpand(true);
        bin.set_vexpand(true);
        bin.append(&initial_pane);

        Rc::new(Self {
            tree: RefCell::new(SplitNode::Leaf {
                pane_widget: initial_pane,
            }),
            bin,
            rebuild_source: RefCell::new(None),
            last_focused: RefCell::new(None),
            state: state.clone(),
        })
    }

    pub(crate) fn new_from_tree(state: &State, node: SplitNode) -> Rc<Self> {
        let bin = gtk::Box::new(gtk::Orientation::Vertical, 0);
        bin.set_hexpand(true);
        bin.set_vexpand(true);

        let widget = build_widget_tree(&node, state);
        bin.append(&widget);

        Rc::new(Self {
            tree: RefCell::new(node),
            bin,
            rebuild_source: RefCell::new(None),
            last_focused: RefCell::new(None),
            state: state.clone(),
        })
    }

    pub(crate) fn widget(&self) -> &gtk::Box {
        &self.bin
    }

    pub(crate) fn tree(&self) -> std::cell::Ref<'_, SplitNode> {
        self.tree.borrow()
    }

    pub(crate) fn is_single_pane(&self) -> bool {
        self.tree.borrow().is_leaf()
    }

    pub(crate) fn split(
        self: &Rc<Self>,
        target: &gtk::Widget,
        new_pane: gtk::Widget,
        orientation: gtk::Orientation,
        new_pane_first: bool,
        ratio: f64,
    ) {
        self.save_focus();
        *self.last_focused.borrow_mut() = Some(new_pane.clone());

        let shared_ratio = Rc::new(RefCell::new(layout_state::clamp_split_ratio(ratio)));
        let new_node = if new_pane_first {
            SplitNode::Split {
                orientation,
                ratio: shared_ratio,
                left: Box::new(SplitNode::Leaf {
                    pane_widget: new_pane,
                }),
                right: Box::new(SplitNode::Leaf {
                    pane_widget: target.clone(),
                }),
            }
        } else {
            SplitNode::Split {
                orientation,
                ratio: shared_ratio,
                left: Box::new(SplitNode::Leaf {
                    pane_widget: target.clone(),
                }),
                right: Box::new(SplitNode::Leaf {
                    pane_widget: new_pane,
                }),
            }
        };

        let replaced = {
            let mut tree = self.tree.borrow_mut();
            tree.replace(target, new_node)
        };

        if replaced {
            self.trigger_rebuild();
        }
    }

    pub(crate) fn remove(self: &Rc<Self>, target: &gtk::Widget) -> bool {
        self.save_focus();

        let removed = {
            let mut tree = self.tree.borrow_mut();
            tree.remove(target)
        };

        if removed {
            self.trigger_rebuild();
        }
        removed
    }

    fn trigger_rebuild(self: &Rc<Self>) {
        if let Some(source) = self.rebuild_source.take() {
            source.remove();
        }

        while let Some(child) = self.bin.first_child() {
            self.bin.remove(&child);
        }

        // Tick separation between unrealize (above) and realize (rebuild) is critical:
        // without it, GLArea contexts in the old tree break.
        self.schedule_rebuild();
    }

    fn schedule_rebuild(self: &Rc<Self>) {
        if self.rebuild_source.borrow().is_some() {
            return;
        }
        let container = Rc::clone(self);
        let source = glib::idle_add_local_once(move || {
            container.rebuild_source.replace(None);
            container.do_rebuild();
        });
        self.rebuild_source.replace(Some(source));
    }

    fn do_rebuild(&self) {
        // Pane widgets may still be parented to old (floating) Paneds from
        // the previous tree. GTK4 won't let us add them to new containers
        // until they're unparented. Detach them all first.
        let tree = self.tree.borrow();
        detach_panes_from_old_tree(&tree);

        let widget = build_widget_tree(&tree, &self.state);
        self.bin.append(&widget);

        if let Some(focused) = self.last_focused.borrow().as_ref() {
            if !pane::focus_active_tab_in_pane(focused) {
                focused.grab_focus();
            }
        }
    }

    fn save_focus(&self) {
        let focus = self
            .bin
            .root()
            .and_then(|r| r.downcast::<gtk::Window>().ok())
            .and_then(|w| gtk::prelude::GtkWindowExt::focus(&w));
        *self.last_focused.borrow_mut() = focus;
    }
}

impl Drop for SplitTreeContainer {
    fn drop(&mut self) {
        if let Some(source) = self.rebuild_source.take() {
            source.remove();
        }
    }
}

fn detach_panes_from_old_tree(node: &SplitNode) {
    match node {
        SplitNode::Leaf { pane_widget } => {
            if let Some(parent) = pane_widget.parent() {
                if let Some(paned) = parent.downcast_ref::<gtk::Paned>() {
                    if paned
                        .start_child()
                        .map(|c| c == *pane_widget)
                        .unwrap_or(false)
                    {
                        paned.set_start_child(gtk::Widget::NONE);
                    } else {
                        paned.set_end_child(gtk::Widget::NONE);
                    }
                }
            }
        }
        SplitNode::Split { left, right, .. } => {
            detach_panes_from_old_tree(left);
            detach_panes_from_old_tree(right);
        }
    }
}

fn build_widget_tree(node: &SplitNode, state: &State) -> gtk::Widget {
    match node {
        SplitNode::Leaf { pane_widget } => pane_widget.clone(),
        SplitNode::Split {
            orientation,
            ratio,
            left,
            right,
        } => {
            let paned = gtk::Paned::builder()
                .orientation(*orientation)
                .hexpand(true)
                .vexpand(true)
                .build();

            let ratio_val = *ratio.borrow();
            update_split_ratio_state(&paned, ratio_val);
            attach_split_position_persistence(state, &paned);

            // Flag suppresses position_notify during programmatic set_position calls
            // (initial layout and workspace re-map). Without this, set_position triggers
            // position_notify which recalculates ratio from not-yet-stable pixel position,
            // corrupting the stored ratio.
            let applying = Rc::new(Cell::new(false));

            let shared_ratio = ratio.clone();
            let applying_for_notify = applying.clone();
            paned.connect_position_notify(move |paned| {
                if applying_for_notify.get() {
                    return;
                }
                let allocation = paned.allocation();
                let size = if paned.orientation() == gtk::Orientation::Horizontal {
                    allocation.width()
                } else {
                    allocation.height()
                };
                let new_ratio = layout_state::snapshot_split_ratio(
                    paned.position(),
                    size,
                    Some(*shared_ratio.borrow()),
                );
                *shared_ratio.borrow_mut() = layout_state::clamp_split_ratio(new_ratio);
            });

            let left_widget = build_widget_tree(left, state);
            let right_widget = build_widget_tree(right, state);
            paned.set_start_child(Some(&left_widget));
            paned.set_end_child(Some(&right_widget));

            apply_split_ratio_after_layout(&paned, *orientation, ratio.clone(), applying);

            paned.upcast()
        }
    }
}

pub(crate) fn build_split_node_from_layout(
    state: &State,
    shortcuts: &Rc<crate::shortcut_config::ResolvedShortcutConfig>,
    ws_id: &str,
    working_directory: Option<&str>,
    layout: &LayoutNodeState,
) -> SplitNode {
    match layout {
        LayoutNodeState::Pane(pane_state) => {
            let pane = crate::window::create_pane_for_workspace(
                state,
                shortcuts,
                ws_id,
                working_directory,
                Some(pane_state),
                false,
            );
            SplitNode::Leaf {
                pane_widget: pane.upcast(),
            }
        }
        LayoutNodeState::Split(split_state) => {
            let orientation = match split_state.orientation {
                SplitOrientation::Horizontal => gtk::Orientation::Horizontal,
                SplitOrientation::Vertical => gtk::Orientation::Vertical,
            };
            SplitNode::Split {
                orientation,
                ratio: Rc::new(RefCell::new(layout_state::clamp_split_ratio(
                    split_state.ratio,
                ))),
                left: Box::new(build_split_node_from_layout(
                    state,
                    shortcuts,
                    ws_id,
                    working_directory,
                    &split_state.start,
                )),
                right: Box::new(build_split_node_from_layout(
                    state,
                    shortcuts,
                    ws_id,
                    working_directory,
                    &split_state.end,
                )),
            }
        }
    }
}
