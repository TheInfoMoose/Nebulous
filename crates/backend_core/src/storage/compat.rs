use std::path::Path;

use toml::{Table, Value};

use crate::error::{Result, TrackerError};

/// Top-level TOML key storing the manifest schema version.
pub const SCHEMA_KEY: &str = "schema_version";

/// Reads `schema_version` from `raw`.
///
/// Returns `1` when the key is missing, not an integer, or less than `1`.
pub fn read_schema_version(raw: &Table) -> u32 {
    match raw.get(SCHEMA_KEY).and_then(Value::as_integer) {
        Some(v) if v >= 1 => u32::try_from(v).unwrap_or(u32::MAX),
        _ => 1,
    }
}

/// Checks that `raw`'s `schema_version` does not exceed `supported`.
///
/// Returns `Err(TrackerError::UnsupportedSchema)` when `found > supported`,
/// otherwise `Ok(found)`.
pub fn check_readable(raw: &Table, path: &Path, supported: u32) -> Result<u32> {
    let found = read_schema_version(raw);
    if found > supported {
        return Err(TrackerError::UnsupportedSchema {
            path: path.to_path_buf(),
            found,
            supported,
        });
    }
    Ok(found)
}

/// Recursively merges keys present in `old` but absent from `new` into `new`.
///
/// For each `(k, v)` in `old`:
/// - If `new` lacks `k`, a clone of `v` is inserted.
/// - If both values are TOML tables, `merge_unknown` recurses into them.
/// - Otherwise `new`'s value is kept.
///
/// Limitation: arrays are never merged element-wise (`new` always wins when an
/// array key exists in both `new` and `old`).
pub fn merge_unknown(new: &mut Table, old: &Table) {
    for (k, old_val) in old {
        match new.get_mut(k) {
            None => {
                new.insert(k.clone(), old_val.clone());
            }
            Some(Value::Table(new_sub)) => {
                if let Value::Table(old_sub) = old_val {
                    merge_unknown(new_sub, old_sub);
                }
            }
            Some(_) => {}
        }
    }
}

/// Stamps or removes `schema_version` on `table`.
///
/// Sets `schema_version = version` when `version > 1 || old_had_key`;
/// otherwise removes `schema_version` so v1 files without the key stay unchanged.
pub fn stamp(table: &mut Table, version: u32, old_had_key: bool) {
    if version > 1 || old_had_key {
        table.insert(SCHEMA_KEY.to_string(), Value::Integer(i64::from(version)));
    } else {
        table.remove(SCHEMA_KEY);
    }
}

/// Migrates a raw TOML table from schema `from` to `to`.
///
/// Returns `table` unchanged when `from >= to`. There are no migrations yet;
/// new migrations go here as `match (from, to)` arms.
pub fn migrate(table: Table, from: u32, to: u32) -> Result<Table> {
    if from >= to {
        return Ok(table);
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn parse_table(src: &str) -> Table {
        toml::from_str(src).expect("valid toml table")
    }

    #[test]
    fn unknown_top_level_key_is_kept() {
        let old = parse_table(
            r#"
            title = "old"
            future_flag = true
            "#,
        );
        let mut new = parse_table(
            r#"
            title = "new"
            "#,
        );
        merge_unknown(&mut new, &old);
        assert_eq!(new.get("title").and_then(Value::as_str), Some("new"));
        assert_eq!(new.get("future_flag").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn unknown_key_inside_nested_table_is_kept() {
        let old = parse_table(
            r#"
            [item]
            id = "BUG-0001"
            future_field = "x"
            "#,
        );
        let mut new = parse_table(
            r#"
            [item]
            id = "BUG-0001"
            status = "closed"
            "#,
        );
        merge_unknown(&mut new, &old);
        let item = new
            .get("item")
            .and_then(Value::as_table)
            .expect("item table");
        assert_eq!(item.get("id").and_then(Value::as_str), Some("BUG-0001"));
        assert_eq!(item.get("status").and_then(Value::as_str), Some("closed"));
        assert_eq!(item.get("future_field").and_then(Value::as_str), Some("x"));
    }

    #[test]
    fn new_value_wins_on_conflict() {
        let old = parse_table(
            r#"
            status = "open"
            [nested]
            count = 1
            "#,
        );
        let mut new = parse_table(
            r#"
            status = "closed"
            [nested]
            count = 2
            "#,
        );
        merge_unknown(&mut new, &old);
        assert_eq!(new.get("status").and_then(Value::as_str), Some("closed"));
        let nested = new.get("nested").and_then(Value::as_table).unwrap();
        assert_eq!(nested.get("count").and_then(Value::as_integer), Some(2));
    }

    #[test]
    fn old_array_replaced_by_new_array() {
        let old = parse_table(
            r#"
            tags = ["a", "b", "c"]
            "#,
        );
        let mut new = parse_table(
            r#"
            tags = ["d"]
            "#,
        );
        merge_unknown(&mut new, &old);
        let tags = new.get("tags").and_then(Value::as_array).unwrap();
        assert_eq!(tags, &vec![Value::String("d".to_string())]);
    }

    #[test]
    fn check_readable_rejects_unsupported_schema() {
        let raw = parse_table("schema_version = 3");
        let path = PathBuf::from(".tracker/project.toml");
        let err = check_readable(&raw, &path, 2).expect_err("3 > 2 must error");
        match err {
            TrackerError::UnsupportedSchema {
                path: p,
                found,
                supported,
            } => {
                assert_eq!(p, path);
                assert_eq!(found, 3);
                assert_eq!(supported, 2);
            }
            other => panic!("unexpected error: {other:?}"),
        }
        assert_eq!(check_readable(&raw, &path, 3).unwrap(), 3);
    }

    #[test]
    fn missing_or_invalid_schema_key_reads_as_one() {
        let missing = parse_table("title = 'hello'");
        assert_eq!(read_schema_version(&missing), 1);

        let zero = parse_table("schema_version = 0");
        assert_eq!(read_schema_version(&zero), 1);

        let negative = parse_table("schema_version = -2");
        assert_eq!(read_schema_version(&negative), 1);

        let non_int = parse_table("schema_version = 'two'");
        assert_eq!(read_schema_version(&non_int), 1);
    }

    #[test]
    fn stamp_v1_without_old_key_leaves_no_key() {
        let mut table = parse_table("schema_version = 1\ntitle = 'a'");
        stamp(&mut table, 1, false);
        assert!(!table.contains_key(SCHEMA_KEY));

        stamp(&mut table, 1, true);
        assert_eq!(table.get(SCHEMA_KEY).and_then(Value::as_integer), Some(1));
    }

    #[test]
    fn stamp_v2_without_old_key_writes_two() {
        let mut table = parse_table("title = 'a'");
        stamp(&mut table, 2, false);
        assert_eq!(table.get(SCHEMA_KEY).and_then(Value::as_integer), Some(2));
    }
}
