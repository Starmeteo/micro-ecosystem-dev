use crate::CoreError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

const MAX_ITEMS: usize = 256;
const SOURCE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
    "/../../apps/desktop/src/render/assets/r5/forest-composed/definition.json"));

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ForestSceneryParent {
    Terrain { id: u32 },
    Scenery { id: u32 },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForestScenery {
    pub id: u32,
    pub style: String,
    /// Root anchor or offset from the parent's anchor, in units of the composed frame.
    pub x: f64,
    pub y: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<ForestSceneryParent>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Definition {
    styles: Vec<Style>,
    initial: Vec<ForestScenery>,
    seed_shell_reference_width_mm: f64,
    seed_shell_placement_y: [f64; 2],
    seed_shell_groups: HashMap<String, SeedShellGroup>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Style {
    id: String,
    #[serde(default)]
    placeable: bool,
    #[serde(default)]
    attached_only: bool,
    #[serde(default)]
    parent_kinds: Vec<ParentKind>,
    seed_shell_group: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedShellGroup {
    span_mm: f64,
}

#[derive(Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
enum ParentKind {
    Terrain,
    Scenery,
}

impl Style {
    fn permits(&self, parent: Option<ForestSceneryParent>) -> bool {
        match parent {
            None => !self.attached_only,
            Some(ForestSceneryParent::Terrain { .. }) => self.parent_kinds.contains(&ParentKind::Terrain),
            Some(ForestSceneryParent::Scenery { .. }) => self.parent_kinds.contains(&ParentKind::Scenery),
        }
    }

    fn accepts_position(&self, x: f64, y: f64, parent: Option<ForestSceneryParent>) -> bool {
        if let Some(group_id) = &self.seed_shell_group {
            let definition = definition();
            let group = &definition.seed_shell_groups[group_id];
            let margin = group.span_mm / (2.0 * definition.seed_shell_reference_width_mm);
            let [top, bottom] = definition.seed_shell_placement_y;
            return parent.is_none() && (margin..=1.0 - margin).contains(&x) && (top..=bottom).contains(&y);
        }
        valid_position(x, y, parent)
    }
}

fn definition() -> &'static Definition {
    static VALUE: OnceLock<Definition> = OnceLock::new();
    VALUE.get_or_init(|| {
        let definition: Definition = serde_json::from_str(SOURCE).expect("valid bundled forest scenery JSON");
        let mut ids = HashSet::new();
        assert!(definition.styles.iter().all(|style| !style.id.is_empty() && ids.insert(&style.id) &&
            style.seed_shell_group.as_ref().is_none_or(|id| definition.seed_shell_groups.contains_key(id))),
            "forest scenery style IDs are unique and seed shell groups exist");
        definition
    })
}

fn valid_position(x: f64, y: f64, parent: Option<ForestSceneryParent>) -> bool {
    // Roots may bleed past the frame; signed offsets cover the whole authored wood surface.
    let extent = if parent.is_some() { -0.5..=0.5 } else { -0.25..=1.25 };
    extent.contains(&x) && extent.contains(&y)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ForestSceneryLayout {
    pub(super) items: Vec<ForestScenery>,
    next_id: u32,
}

impl ForestSceneryLayout {
    pub(super) fn empty() -> Self {
        Self { items: Vec::new(), next_id: 1 }
    }

    pub(super) fn seeded() -> Self {
        let items = definition().initial.clone();
        let next_id = items.iter().map(|item| item.id).max().unwrap_or(0)
            .checked_add(1).expect("bundled forest scenery has allocatable IDs");
        Self { items, next_id }
    }

    pub(super) fn preserve_allocations(&mut self, current: &Self) {
        self.next_id = self.next_id.max(current.next_id);
    }

    pub(super) fn valid(&self, terrain_exists: impl Fn(u32) -> bool) -> bool {
        if self.next_id == 0 || self.items.len() > MAX_ITEMS { return false; }
        let by_id: HashMap<_, _> = self.items.iter().map(|item| (item.id, item)).collect();
        by_id.len() == self.items.len() && self.items.iter().all(|item| {
            if item.id == 0 || item.id >= self.next_id || !definition().styles.iter().any(|style| {
                style.id == item.style && style.permits(item.parent) && style.accepts_position(item.x, item.y, item.parent)
            }) {
                return false;
            }
            let mut parent = item.parent;
            // The walk must reach a root before it can repeat a scenery ID.
            for _ in 0..by_id.len() {
                match parent {
                    None => return true,
                    Some(ForestSceneryParent::Terrain { id }) => return terrain_exists(id),
                    Some(ForestSceneryParent::Scenery { id }) => match by_id.get(&id) {
                        Some(entry) => parent = entry.parent,
                        None => return false,
                    },
                }
            }
            false
        })
    }

    pub(super) fn add(&mut self, style: &str, x: f64, y: f64, parent: Option<ForestSceneryParent>) -> Result<(), CoreError> {
        if !definition().styles.iter().any(|entry| {
            entry.id == style && entry.placeable && entry.permits(parent) && entry.accepts_position(x, y, parent)
        }) {
            return Err(CoreError::InvalidForestScenery);
        }
        if self.items.len() >= MAX_ITEMS { return Err(CoreError::ForestSceneryLimit); }
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or(CoreError::ForestSceneryLimit)?;
        self.items.push(ForestScenery { id, style: style.to_owned(), x, y, parent });
        Ok(())
    }

    pub(super) fn move_item(&mut self, id: u32, x: f64, y: f64) -> Result<(), CoreError> {
        let item = self.items.iter_mut().find(|item| item.id == id).ok_or(CoreError::ForestSceneryMissing)?;
        if !definition().styles.iter().any(|style| style.id == item.style && style.accepts_position(x, y, item.parent)) {
            return Err(CoreError::InvalidForestScenery);
        }
        item.x = x;
        item.y = y;
        Ok(())
    }

    pub(super) fn remove(&mut self, id: u32) -> Result<(), CoreError> {
        let index = self.items.iter().position(|item| item.id == id).ok_or(CoreError::ForestSceneryMissing)?;
        self.items.remove(index);
        self.remove_children(ForestSceneryParent::Scenery { id });
        Ok(())
    }

    pub(super) fn remove_children(&mut self, parent: ForestSceneryParent) {
        let mut pending = vec![parent];
        while let Some(parent) = pending.pop() {
            self.items.retain(|item| {
                if item.parent != Some(parent) { return true; }
                pending.push(ForestSceneryParent::Scenery { id: item.id });
                false
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ForestRockEdit, WorldState};

    #[test]
    fn r5_seed_shells_are_ground_decorations_without_resource_or_simulation_changes() {
        let world = WorldState::new_blank_r5_composed_forest(804);
        let mut decorated = world.clone();
        let before = decorated.forest_layout_edit_state().unwrap();
        for shell in ["seed-shell", "acorn-shell"] {
            for density in ["sparse", "loose", "dense"] {
                let style = format!("{shell}-{density}");
                decorated.edit_forest_rocks(&ForestRockEdit::AddScenery { style, x: 0.55, y: 0.74, parent: None }).unwrap();
            }
        }
        let items = decorated.forest_rock_layout().unwrap().scenery().unwrap();
        assert_eq!(items.len(), 6);
        assert!(items.iter().all(|item| item.parent.is_none()));
        let id = items[2].id;
        let decorated_before_invalid = decorated.clone();
        assert_eq!(decorated.edit_forest_rocks(&ForestRockEdit::MoveScenery { id, x: 0.03, y: 0.74 }), Err(CoreError::InvalidForestScenery));
        assert_eq!(decorated.edit_forest_rocks(&ForestRockEdit::AddScenery {
            style: "acorn-shell-dense".to_owned(), x: 0.50, y: 0.20, parent: None,
        }), Err(CoreError::InvalidForestScenery));
        assert_eq!(decorated.edit_forest_rocks(&ForestRockEdit::AddScenery {
            style: "acorn-shell-dense".to_owned(), x: 0.50, y: 0.74, parent: Some(ForestSceneryParent::Scenery { id }),
        }), Err(CoreError::InvalidForestScenery));
        assert_eq!(decorated, decorated_before_invalid);

        decorated.edit_forest_rocks(&ForestRockEdit::MoveScenery { id, x: 0.36, y: 0.86 }).unwrap();
        let moved = decorated.clone();
        let after = decorated.forest_layout_edit_state().unwrap();
        decorated.restore_forest_layout_edit_state(before).unwrap();
        assert!(decorated.forest_rock_layout().unwrap().scenery().unwrap().is_empty());
        decorated.restore_forest_layout_edit_state(after).unwrap();
        assert_eq!(decorated, moved);
        let encoded = serde_json::to_vec(&decorated).unwrap();
        let reopened: WorldState = serde_json::from_slice(&encoded).unwrap();
        reopened.validate().unwrap();
        assert_eq!(reopened, decorated);
        assert_eq!(decorated.forest_ledger(), world.forest_ledger());

        let mut comparison = world;
        decorated.advance_steps(70).unwrap();
        comparison.advance_steps(70).unwrap();
        assert_eq!(decorated.forest_ledger(), comparison.forest_ledger());
        assert_eq!(decorated.animals(), comparison.animals());
        decorated.edit_forest_rocks(&ForestRockEdit::RemoveScenery { id }).unwrap();
        assert_eq!(decorated.forest_ledger(), comparison.forest_ledger());
        assert!(!decorated.forest_rock_layout().unwrap().scenery().unwrap().iter().any(|item| item.id == id));
    }
}
