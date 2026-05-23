use crate::cursor::UzCursorIcon;
use crate::node::UzNodeId;

use crate::style::{Bounds, ScrollbarStyle, UzStyleRefinement};
use vello::kurbo::{Affine, Point};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HitboxId(pub u64);

#[derive(Clone, Debug)]
pub struct Hitbox {
    pub id: HitboxId,
    pub node_id: UzNodeId,
    /// Axis-aligned logical bounds kept for legacy geometry consumers.
    pub bounds: Bounds,
    /// The node-local hit region before transform.
    pub local_bounds: Bounds,
    /// Logical-space transform from local node coordinates to window coordinates.
    pub transform: Affine,
}

impl Hitbox {
    /// Check if this hitbox is hovered according to the current hit test result.
    pub fn is_hovered(&self, hit_state: &HitTestState) -> bool {
        hit_state.is_hovered(self.node_id)
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        let local = self.transform.inverse() * Point::new(x, y);
        self.local_bounds.contains(local.x, local.y)
    }
}

/// Stores the result of a hit test: which hitboxes the mouse is currently over.
#[derive(Clone, Debug, Default)]
pub struct HitTestState {
    /// Mouse position in window coordinates.
    pub mouse_position: Option<(f64, f64)>,
    /// Set of node IDs that the mouse is currently over (back-to-front order).
    pub hovered_nodes: Vec<UzNodeId>,
    /// The topmost (frontmost) hovered node, if any.
    pub top_node: Option<UzNodeId>,
    /// Which node is currently pressed (mouse down without mouse up).
    pub active_node: Option<UzNodeId>,
}

impl HitTestState {
    pub fn is_hovered(&self, node_id: UzNodeId) -> bool {
        self.hovered_nodes.contains(&node_id)
    }

    pub fn is_active(&self, node_id: UzNodeId) -> bool {
        self.active_node == Some(node_id) && self.is_hovered(node_id)
    }
}

/// Stores all hitboxes registered during a paint pass. Order matters (back to front).
#[derive(Clone, Debug, Default)]
pub struct HitboxStore {
    hitboxes: Vec<Hitbox>,
    next_id: u64,
}

impl HitboxStore {
    pub fn clear(&mut self) {
        self.hitboxes.clear();
        self.next_id = 0;
    }

    /// Drop any hitbox whose `node_id` no longer passes `keep`.
    /// Used by Dom::on_node_removed to scrub stale references after a node is freed.
    pub fn retain_by_node(&mut self, mut keep: impl FnMut(UzNodeId) -> bool) {
        self.hitboxes.retain(|h| keep(h.node_id));
    }

    /// Register a hitbox and return its ID.
    pub fn insert(&mut self, node_id: UzNodeId, bounds: Bounds) -> HitboxId {
        self.insert_transformed(node_id, bounds, Affine::IDENTITY)
    }

    pub fn insert_transformed(
        &mut self,
        node_id: UzNodeId,
        local_bounds: Bounds,
        transform: Affine,
    ) -> HitboxId {
        let id = HitboxId(self.next_id);
        self.next_id += 1;
        let bounds = transformed_axis_aligned_bounds(local_bounds, transform);
        self.hitboxes.push(Hitbox {
            id,
            node_id,
            bounds,
            local_bounds,
            transform,
        });
        id
    }

    /// Get a hitbox by its ID.
    pub fn get(&self, id: HitboxId) -> Option<&Hitbox> {
        self.hitboxes.iter().find(|h| h.id == id)
    }

    /// Run a hit test at the given position. Walk hitboxes back-to-front
    /// (last registered = frontmost) and return all that contain the point.
    pub fn hit_test(&self, x: f64, y: f64) -> HitTestState {
        let mut hovered = Vec::new();
        let mut top_node = None;

        // Walk back-to-front: later entries are painted on top
        for hitbox in self.hitboxes.iter().rev() {
            if hitbox.contains(x, y) {
                if top_node.is_none() {
                    top_node = Some(hitbox.node_id);
                }
                if !hovered.contains(&hitbox.node_id) {
                    hovered.push(hitbox.node_id);
                }
            }
        }

        // Reverse so order is back-to-front (matching paint order)
        hovered.reverse();

        HitTestState {
            mouse_position: Some((x, y)),
            hovered_nodes: hovered,
            top_node,
            active_node: None, // Caller must preserve active state
        }
    }

    pub fn hitboxes(&self) -> &[Hitbox] {
        &self.hitboxes
    }
}

fn transformed_axis_aligned_bounds(bounds: Bounds, transform: Affine) -> Bounds {
    let points = [
        transform * Point::new(bounds.x, bounds.y),
        transform * Point::new(bounds.x + bounds.width, bounds.y),
        transform * Point::new(bounds.x + bounds.width, bounds.y + bounds.height),
        transform * Point::new(bounds.x, bounds.y + bounds.height),
    ];

    let (mut min_x, mut min_y) = (points[0].x, points[0].y);
    let (mut max_x, mut max_y) = (points[0].x, points[0].y);
    for point in points.iter().skip(1) {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }

    Bounds::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum StyleSlot {
    Base,
    Hover,
    Active,
    Focus,
}

#[derive(Default)]
pub struct Interactivity {
    pub cursor: Option<UzCursorIcon>,

    pub base_style: Box<UzStyleRefinement>,
    pub hover_style: Option<Box<UzStyleRefinement>>,
    pub active_style: Option<Box<UzStyleRefinement>>,
    pub focus_style: Option<Box<UzStyleRefinement>>,

    // not used yet
    pub scrollbar: ScrollbarStyle,
}

impl Interactivity {
    pub(crate) fn style_for(&mut self, variant: StyleSlot) -> &mut UzStyleRefinement {
        match variant {
            StyleSlot::Hover => self
                .hover_style
                .get_or_insert_with(|| Box::new(UzStyleRefinement::default())),
            StyleSlot::Active => self
                .active_style
                .get_or_insert_with(|| Box::new(UzStyleRefinement::default())),
            StyleSlot::Focus => self
                .focus_style
                .get_or_insert_with(|| Box::new(UzStyleRefinement::default())),
            StyleSlot::Base => &mut self.base_style,
        }
    }
}
