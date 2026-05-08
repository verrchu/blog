use std::str::FromStr;

use anyhow::Context as _;
use yaml_edit::Document;

const ROOT: &str = "asset_groups";
const DEFAULT: &str = "default";

pub fn list_assets(input: &str, assets: &[String]) -> anyhow::Result<String> {
    let doc = Document::from_str(input).context("parse")?;
    let root = doc.as_mapping().context("root is not a mapping")?;
    let groups = root.get_mapping(ROOT).context("no `asset_groups` mapping")?;

    let mut existing: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (key, _) in &groups {
        let group_name = key.to_string();
        if let Some(seq) = groups.get_sequence(&group_name) {
            for item in seq.values() {
                if let Some(scalar) = item.as_scalar() {
                    existing.insert(scalar.as_string());
                }
            }
        }
    }

    let to_add: Vec<&String> = assets.iter().filter(|a| !existing.contains(*a)).collect();
    if to_add.is_empty() {
        return Ok(doc.to_string());
    }

    let default_seq = groups
        .get_sequence(DEFAULT)
        .context("no `default` group")?;
    let mut current: Vec<String> = default_seq
        .values()
        .filter_map(|v| v.as_scalar().map(yaml_edit::Scalar::as_string))
        .collect();
    current.extend(to_add.into_iter().cloned());
    current.sort();

    default_seq.clear();
    for item in &current {
        default_seq.push(item.as_str());
    }

    Ok(doc.to_string())
}

pub fn delist_assets(input: &str, assets: &[String]) -> anyhow::Result<String> {
    let doc = Document::from_str(input).context("parse")?;
    let root = doc.as_mapping().context("root is not a mapping")?;
    let groups = root.get_mapping(ROOT).context("no `asset_groups` mapping")?;

    let to_remove: std::collections::HashSet<&str> =
        assets.iter().map(String::as_str).collect();

    let group_names: Vec<String> = groups.keys().map(|k| k.to_string()).collect();
    for group in &group_names {
        let Some(seq) = groups.get_sequence(group) else {
            continue;
        };
        let kept: Vec<String> = seq
            .values()
            .filter_map(|v| v.as_scalar().map(yaml_edit::Scalar::as_string))
            .filter(|a| !to_remove.contains(a.as_str()))
            .collect();

        let original_len = seq.len();
        if kept.len() == original_len {
            continue;
        }
        if kept.is_empty() {
            groups.remove(group.as_str());
        } else {
            seq.clear();
            for item in &kept {
                seq.push(item.as_str());
            }
        }
    }

    Ok(doc.to_string())
}
