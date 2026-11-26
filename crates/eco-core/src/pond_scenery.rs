use crate::CoreError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

const MAX_ITEMS: usize = 256;
const DEFINITION: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
    "/../../apps/desktop/src/render/assets/r5/pond-composed/definition.json"));

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PondSceneryParent {
    Wood { id: u32 },
    Scenery { id: u32 },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PondScenery {
    pub id: u32,
    pub style: String,
    /// Normalized scene anchor for roots, normalized parent-local anchor for children.
    pub x: f64,
    pub y: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<PondSceneryParent>,
}

#[derive(Deserialize)]
struct Definition {
    styles: Vec<Style>,
    initial: Vec<PondScenery>,
}

#[derive(Deserialize)]
struct Style {
    id: String,
    #[serde(default)]
    placeable: bool,
    #[serde(default, rename = "bedAnchor")]
    bed_anchor: Option<BedAnchor>,
}

#[derive(Deserialize)]
struct BedAnchor {
    x: [f64; 2],
    y: [f64; 2],
}

impl Style {
    fn valid_position(&self, x: f64, y: f64, parent: Option<PondSceneryParent>) -> bool {
        if let Some(bounds) = &self.bed_anchor {
            return parent.is_none() && (bounds.x[0]..=bounds.x[1]).contains(&x) && (bounds.y[0]..=bounds.y[1]).contains(&y);
        }
        let extent = if parent.is_some() { 0.0..=1.0 } else { -0.125..=1.125 };
        extent.contains(&x) && extent.contains(&y)
    }
}

fn definition() -> &'static Definition {
    static VALUE: OnceLock<Definition> = OnceLock::new();
    VALUE.get_or_init(|| {
        let definition: Definition = serde_json::from_str(DEFINITION).expect("valid bundled pond scenery JSON");
        let mut ids = HashSet::new();
        assert!(definition.styles.iter().all(|style| !style.id.is_empty() && ids.insert(&style.id)),
            "pond scenery style IDs are unique and non-empty");
        definition
    })
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PondSceneryLayout {
    pub(super) items: Vec<PondScenery>,
    next_id: u32,
}

impl PondSceneryLayout {
    pub(super) fn empty() -> Self {
        Self { items: Vec::new(), next_id: 1 }
    }

    pub(super) fn seeded(wood_id: u32) -> Self {
        let items = definition().initial.clone();
        let next_id = items.iter().map(|item| item.id).max().unwrap_or(0)
            .checked_add(1).expect("bundled pond scenery has allocatable IDs");
        let layout = Self { items, next_id };
        assert!(layout.valid(|id| id == wood_id), "valid bundled pond scenery layout");
        layout
    }

    pub(super) fn preserve_allocations(&mut self, current: &Self) {
        self.next_id = self.next_id.max(current.next_id);
    }

    pub(super) fn valid(&self, wood_exists: impl Fn(u32) -> bool) -> bool {
        if self.next_id == 0 || self.items.len() > MAX_ITEMS { return false; }
        let by_id: HashMap<_, _> = self.items.iter().map(|item| (item.id, item)).collect();
        by_id.len() == self.items.len() && self.items.iter().all(|item| {
            if item.id == 0 || item.id >= self.next_id
                || !Self::style(&item.style).is_some_and(|style| style.valid_position(item.x, item.y, item.parent)) {
                return false;
            }
            let mut parent = item.parent;
            // Each valid chain reaches a root or the physical wood in at most one pass over the IDs.
            for _ in 0..by_id.len() {
                match parent {
                    None => return true,
                    Some(PondSceneryParent::Wood { id }) => return wood_exists(id),
                    Some(PondSceneryParent::Scenery { id }) => match by_id.get(&id) {
                        Some(entry) => parent = entry.parent,
                        None => return false,
                    },
                }
            }
            false
        })
    }

    fn style(style: &str) -> Option<&'static Style> {
        definition().styles.iter().find(|entry| entry.id == style)
    }

    pub(super) fn add(&mut self, style: &str, x: f64, y: f64) -> Result<(), CoreError> {
        if !Self::style(style).is_some_and(|entry| entry.placeable && entry.valid_position(x, y, None)) {
            return Err(CoreError::InvalidPondScenery);
        }
        if self.items.len() >= MAX_ITEMS { return Err(CoreError::PondSceneryLimit); }
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or(CoreError::PondSceneryLimit)?;
        self.items.push(PondScenery { id, style: style.to_owned(), x, y, parent: None });
        Ok(())
    }

    pub(super) fn move_item(&mut self, id: u32, x: f64, y: f64) -> Result<(), CoreError> {
        let item = self.items.iter_mut().find(|item| item.id == id).ok_or(CoreError::PondSceneryMissing)?;
        if !Self::style(&item.style).is_some_and(|style| style.valid_position(x, y, item.parent)) {
            return Err(CoreError::InvalidPondScenery);
        }
        item.x = x;
        item.y = y;
        Ok(())
    }

    pub(super) fn remove(&mut self, id: u32) -> Result<(), CoreError> {
        let index = self.items.iter().position(|item| item.id == id).ok_or(CoreError::PondSceneryMissing)?;
        self.items.remove(index);
        self.remove_children(PondSceneryParent::Scenery { id });
        Ok(())
    }

    pub(super) fn remove_children(&mut self, parent: PondSceneryParent) {
        let mut pending = vec![parent];
        while let Some(parent) = pending.pop() {
            self.items.retain(|item| {
                if item.parent != Some(parent) { return true; }
                pending.push(PondSceneryParent::Scenery { id: item.id });
                false
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_cycles_or_dangling_parents_and_removes_all_nested_children() {
        let style = definition().styles.iter().find(|style| style.placeable).expect("placeable scenery style").id.clone();
        let items = [None, Some(PondSceneryParent::Wood { id: 7 }),
            Some(PondSceneryParent::Scenery { id: 2 }), Some(PondSceneryParent::Scenery { id: 3 })]
            .into_iter().enumerate().map(|(index, parent)| PondScenery {
                id: index as u32 + 1, style: style.clone(), x: 0.5, y: 0.5, parent,
            }).collect();
        let mut layout = PondSceneryLayout { items, next_id: 5 };
        assert!(layout.valid(|id| id == 7));
        assert!(!layout.valid(|_| false));
        layout.items[1].parent = Some(PondSceneryParent::Scenery { id: 4 });
        assert!(!layout.valid(|_| true));
        layout.items[1].parent = Some(PondSceneryParent::Wood { id: 7 });
        assert_eq!(layout.move_item(3, 1.05, 0.5), Err(CoreError::InvalidPondScenery));
        assert_eq!(layout.items[2].x, 0.5);
        let mut removing_root = layout.clone();
        removing_root.remove(2).unwrap();
        assert_eq!(removing_root.items, layout.items[..1]);
        layout.remove_children(PondSceneryParent::Wood { id: 7 });
        assert_eq!(layout.items.iter().map(|item| item.id).collect::<Vec<_>>(), vec![1]);
        layout.add(&style, -0.125, 1.125).unwrap();
        assert_eq!(layout.items[1].id, 5);
    }
}
