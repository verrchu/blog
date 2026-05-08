// Naive `Op::Replace(Sequence(...))` against a block-sequence route fails with
// "input is not valid YAML" inside yamlpatch 1.24, so list rewrites use the
// append-new-then-pop-old-from-index-0 dance instead.
use std::collections::BTreeMap;

use anyhow::Context as _;
use yamlpatch::{Op, Patch, apply_yaml_patches};

const DEFAULT: &str = "default";

pub fn list_assets(input: &str, assets: &[String]) -> anyhow::Result<String> {
    let doc = yamlpath::Document::new(input.to_string()).context("parse")?;

    let groups = read_groups(&doc)?;
    let existing: std::collections::HashSet<String> =
        groups.values().flatten().cloned().collect();

    let to_add: Vec<&String> = assets.iter().filter(|a| !existing.contains(*a)).collect();
    if to_add.is_empty() {
        return Ok(doc.source().to_string());
    }

    let default_old = groups.get(DEFAULT).cloned().unwrap_or_default();
    let mut default_new = default_old.clone();
    default_new.extend(to_add.into_iter().cloned());
    default_new.sort();

    let new_doc = rewrite_list(&doc, DEFAULT, default_old.len(), &default_new)?;
    Ok(new_doc.source().to_string())
}

pub fn delist_assets(input: &str, assets: &[String]) -> anyhow::Result<String> {
    let doc = yamlpath::Document::new(input.to_string()).context("parse")?;
    let groups = read_groups(&doc)?;

    let to_remove: std::collections::HashSet<&str> =
        assets.iter().map(String::as_str).collect();

    let mut new_doc = doc;
    for (group, items) in &groups {
        let kept: Vec<String> = items
            .iter()
            .filter(|a| !to_remove.contains(a.as_str()))
            .cloned()
            .collect();
        if kept.len() == items.len() {
            continue;
        }
        new_doc = if kept.is_empty() {
            let patch = Patch {
                route: yamlpath::route!("asset_groups", group.as_str()),
                operation: Op::Remove,
            };
            apply_yaml_patches(&new_doc, &[patch]).context("remove empty group")?
        } else {
            rewrite_list(&new_doc, group, items.len(), &kept)?
        };
    }

    Ok(new_doc.source().to_string())
}

fn rewrite_list(
    doc: &yamlpath::Document,
    group: &str,
    old_len: usize,
    new_items: &[String],
) -> anyhow::Result<yamlpath::Document> {
    let mut patches: Vec<Patch> = Vec::with_capacity(old_len + new_items.len());
    for item in new_items {
        patches.push(Patch {
            route: yamlpath::route!("asset_groups", group),
            operation: Op::Append {
                value: serde_yaml::Value::String(item.clone()),
            },
        });
    }
    for _ in 0..old_len {
        patches.push(Patch {
            route: yamlpath::route!("asset_groups", group, 0usize),
            operation: Op::Remove,
        });
    }
    apply_yaml_patches(doc, &patches).context("apply patches")
}

fn read_groups(doc: &yamlpath::Document) -> anyhow::Result<BTreeMap<String, Vec<String>>> {
    let raw: serde_yaml::Value =
        serde_yaml::from_str(doc.source()).context("parse with serde_yaml")?;
    let groups = raw
        .get("asset_groups")
        .context("no `asset_groups` key")?
        .as_mapping()
        .context("`asset_groups` is not a mapping")?;

    let mut out = BTreeMap::new();
    for (k, v) in groups {
        let key = k.as_str().context("non-string group name")?.to_string();
        let seq = v
            .as_sequence()
            .with_context(|| format!("group `{key}` is not a sequence"))?;
        let items: Vec<String> = seq
            .iter()
            .map(|i| {
                i.as_str()
                    .context("asset is not a string")
                    .map(str::to_string)
            })
            .collect::<anyhow::Result<_>>()?;
        out.insert(key, items);
    }
    Ok(out)
}
