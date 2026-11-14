use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::models::item::{ItemStatus, ItemType, Severity};

/// Tier 3: Domain/crate scope index manifest (`.../domains/<scope_id>/scope.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeManifest {
    pub scope: ScopeMetadata,
    #[serde(default)]
    pub items: ScopeItems,
}

impl ScopeManifest {
    /// Deserializes a `ScopeManifest` from a TOML string.
    pub fn from_toml_str(input: &str) -> Result<Self> {
        Ok(toml::from_str(input)?)
    }

    /// Serializes this `ScopeManifest` into a formatted TOML string.
    pub fn to_toml_string(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Returns total number of indexed bugs and features in this scope.
    pub fn total_items(&self) -> usize {
        self.items.iter().count()
    }
}

/// `[scope]` table inside `scope.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeMetadata {
    pub id: String,
    pub target: String,
    #[serde(default)]
    pub tech_stack: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_component: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracked_files: Vec<String>,
}

/// `[items]` table inside `scope.toml` containing lightweight item stubs for fast index loading.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeItems {
    #[serde(default)]
    pub bugs: Vec<ItemStub>,
    #[serde(default)]
    pub features: Vec<ItemStub>,
}

impl ScopeItems {
    /// Iterates over all indexed stubs tagged with their `ItemType` (bugs, then features, in stored order).
    pub fn iter(&self) -> impl Iterator<Item = (ItemType, &ItemStub)> {
        self.bugs
            .iter()
            .map(|stub| (ItemType::Bug, stub))
            .chain(self.features.iter().map(|stub| (ItemType::Feature, stub)))
    }

    /// Returns the number of stubs stored for `kind`.
    pub fn count(&self, kind: ItemType) -> usize {
        match kind {
            ItemType::Bug => self.bugs.len(),
            ItemType::Feature => self.features.len(),
        }
    }

    /// Appends `stub` to `kind`'s list and sorts that list with `ItemStub::chronological_cmp`.
    ///
    /// Does not deduplicate; callers that move or update an existing stub must call `remove_id` first.
    pub fn insert(&mut self, kind: ItemType, stub: ItemStub) {
        let list = match kind {
            ItemType::Bug => &mut self.bugs,
            ItemType::Feature => &mut self.features,
        };
        list.push(stub);
        list.sort_by(ItemStub::chronological_cmp);
    }

    /// Removes any stub with `id` from every kind list.
    pub fn remove_id(&mut self, id: &str) {
        self.bugs.retain(|s| s.id != id);
        self.features.retain(|s| s.id != id);
    }
}

/// Lightweight item summary stored in `scope.toml` `[items]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStub {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub status: ItemStatus,
    #[serde(default)]
    pub severity: Severity,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub created_at: String,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum CreatedAtRank<'a> {
    Empty,
    Parsed(chrono::DateTime<chrono::Utc>),
    Unparseable(&'a str),
}

fn created_at_rank(value: &str) -> CreatedAtRank<'_> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        CreatedAtRank::Empty
    } else if let Ok(dt) = crate::models::parse_timestamp_utc(trimmed) {
        CreatedAtRank::Parsed(dt)
    } else {
        CreatedAtRank::Unparseable(trimmed)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum IdRank<'a> {
    Sequential(u8, u32, &'a str),
    Other(&'a str),
}

fn id_rank(id: &str) -> IdRank<'_> {
    if let Some((item_type, number)) = crate::storage::parse_item_id_number(id) {
        let type_order = item_type.ordinal();
        IdRank::Sequential(type_order, number, id)
    } else {
        IdRank::Other(id)
    }
}

/// Total order on item IDs: sequential IDs (by type ordinal, then number, then ID string) before all other IDs (by string).
pub(crate) fn cmp_item_ids(a: &str, b: &str) -> std::cmp::Ordering {
    id_rank(a).cmp(&id_rank(b))
}

impl ItemStub {
    /// Orders item stubs chronologically by `created_at` instant (empty first, then parsed UTC
    /// instant, then unparseable trimmed string), breaking ties with sequential IDs first
    /// (by type, then number, then ID string), then all other IDs by string.
    pub fn chronological_cmp(&self, other: &Self) -> std::cmp::Ordering {
        created_at_rank(&self.created_at)
            .cmp(&created_at_rank(&other.created_at))
            .then_with(|| cmp_item_ids(&self.id, &other.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub(id: &str, created_at: &str) -> ItemStub {
        ItemStub {
            id: id.to_string(),
            title: "t".to_string(),
            status: ItemStatus::default(),
            severity: Severity::default(),
            created_at: created_at.to_string(),
        }
    }

    #[test]
    fn scope_items_insert_keeps_chronological_order() {
        let mut items = ScopeItems::default();
        items.insert(ItemType::Bug, stub("BUG-0002", "2026-01-02T00:00:00Z"));
        items.insert(ItemType::Bug, stub("BUG-0001", "2026-01-01T00:00:00Z"));

        let bug_ids: Vec<&str> = items.bugs.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(bug_ids, vec!["BUG-0001", "BUG-0002"]);
        assert!(items.features.is_empty());
    }

    #[test]
    fn scope_items_remove_id_drops_from_every_list() {
        let mut items = ScopeItems {
            bugs: vec![stub("BUG-0001", "2026-01-01T00:00:00Z")],
            features: vec![
                stub("FEAT-0001", "2026-01-01T00:00:00Z"),
                stub("FEAT-0002", "2026-01-02T00:00:00Z"),
            ],
        };

        items.remove_id("FEAT-0001");
        let bug_ids: Vec<&str> = items.bugs.iter().map(|s| s.id.as_str()).collect();
        let feat_ids: Vec<&str> = items.features.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(bug_ids, vec!["BUG-0001"]);
        assert_eq!(feat_ids, vec!["FEAT-0002"]);

        items.remove_id("NOPE");
        assert_eq!(items.bugs.len(), 1);
        assert_eq!(items.features.len(), 1);
    }

    #[test]
    fn scope_items_iter_and_count_cover_every_kind() {
        let items = ScopeItems {
            bugs: vec![
                stub("BUG-0001", "2026-01-01T00:00:00Z"),
                stub("BUG-0002", "2026-01-02T00:00:00Z"),
            ],
            features: vec![stub("FEAT-0001", "2026-01-03T00:00:00Z")],
        };

        let collected: Vec<(ItemType, &str)> =
            items.iter().map(|(k, s)| (k, s.id.as_str())).collect();
        assert_eq!(
            collected,
            vec![
                (ItemType::Bug, "BUG-0001"),
                (ItemType::Bug, "BUG-0002"),
                (ItemType::Feature, "FEAT-0001"),
            ]
        );
        assert_eq!(items.count(ItemType::Bug), 2);
        assert_eq!(items.count(ItemType::Feature), 1);

        let manifest = ScopeManifest {
            scope: ScopeMetadata::default(),
            items,
        };
        assert_eq!(manifest.total_items(), 3);
    }

    #[test]
    fn cmp_item_ids_is_a_total_order_over_mixed_ids() {
        use std::cmp::Ordering;

        let ids = [
            "BUG-9999",
            "BUG-10000",
            "BUG-5ABCDE",
            "FEAT-0001",
            "bug-0002",
            "BUG-ABCDEF",
            "NOTANID",
        ];

        assert_eq!(cmp_item_ids("BUG-9999", "BUG-10000"), Ordering::Less);
        assert_eq!(cmp_item_ids("BUG-10000", "BUG-5ABCDE"), Ordering::Less);
        assert_eq!(cmp_item_ids("BUG-9999", "BUG-5ABCDE"), Ordering::Less);

        for a in ids {
            assert_eq!(cmp_item_ids(a, a), Ordering::Equal);
            for b in ids {
                assert_eq!(cmp_item_ids(a, b), cmp_item_ids(b, a).reverse());
                for c in ids {
                    if cmp_item_ids(a, b) != Ordering::Greater
                        && cmp_item_ids(b, c) != Ordering::Greater
                    {
                        assert_ne!(cmp_item_ids(a, c), Ordering::Greater);
                    }
                }
            }
        }
    }
}
