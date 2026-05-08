use std::borrow::Cow;
use std::collections::BTreeMap;

use anyhow::Context as _;
use yamp::{YamlNode, YamlValue, emit, parse};

const ROOT: &str = "asset_groups";
const DEFAULT: &str = "default";

pub fn list_assets(input: &str, assets: &[String]) -> anyhow::Result<String> {
    let parsed = parse(input).map_err(|e| anyhow::anyhow!("parse: {e}"))?;
    let mut doc = into_static(parsed);

    let groups = root_mut(&mut doc)?;
    let existing: std::collections::HashSet<String> = groups
        .values()
        .filter_map(|n| match &n.value {
            YamlValue::Array(items) => Some(items),
            _ => None,
        })
        .flat_map(|items| items.iter().filter_map(|n| n.as_str().map(str::to_string)))
        .collect();

    let to_add: Vec<&String> = assets.iter().filter(|a| !existing.contains(*a)).collect();
    if to_add.is_empty() {
        return Ok(emit(&doc));
    }

    let default_node = groups
        .get_mut(&Cow::Borrowed(DEFAULT))
        .context("no `default` group")?;
    let YamlValue::Array(default_items) = &mut default_node.value else {
        anyhow::bail!("`default` is not an array");
    };
    for asset in to_add {
        default_items.push(YamlNode::from_value(YamlValue::String(Cow::Owned(
            asset.clone(),
        ))));
    }
    default_items.sort_by(|a, b| {
        a.as_str()
            .unwrap_or("")
            .cmp(b.as_str().unwrap_or(""))
    });

    Ok(emit(&doc))
}

pub fn delist_assets(input: &str, assets: &[String]) -> anyhow::Result<String> {
    let parsed = parse(input).map_err(|e| anyhow::anyhow!("parse: {e}"))?;
    let mut doc = into_static(parsed);

    let groups = root_mut(&mut doc)?;
    let to_remove: std::collections::HashSet<&str> =
        assets.iter().map(String::as_str).collect();

    let group_keys: Vec<Cow<'static, str>> = groups.keys().cloned().collect();
    let mut empty: Vec<Cow<'static, str>> = Vec::new();
    for k in &group_keys {
        let Some(node) = groups.get_mut(k) else {
            continue;
        };
        let YamlValue::Array(items) = &mut node.value else {
            continue;
        };
        items.retain(|n| match n.as_str() {
            Some(s) => !to_remove.contains(s),
            None => true,
        });
        if items.is_empty() {
            empty.push(k.clone());
        }
    }
    for k in empty {
        groups.remove(&k);
    }

    Ok(emit(&doc))
}

fn root_mut<'a>(
    doc: &'a mut YamlNode<'static>,
) -> anyhow::Result<&'a mut BTreeMap<Cow<'static, str>, YamlNode<'static>>> {
    let YamlValue::Object(top) = &mut doc.value else {
        anyhow::bail!("root is not an object");
    };
    let groups_node = top
        .get_mut(&Cow::Borrowed(ROOT))
        .context("no `asset_groups` key")?;
    match &mut groups_node.value {
        YamlValue::Object(m) => {
            for (k, v) in m.iter() {
                if matches!(v.value, YamlValue::String(_)) {
                    anyhow::bail!("yamp parsed group `{k}` as a scalar, not a sequence");
                }
            }
            Ok(m)
        }
        _ => anyhow::bail!("`asset_groups` is not an object"),
    }
}

fn into_static(n: YamlNode<'_>) -> YamlNode<'static> {
    YamlNode {
        value: match n.value {
            YamlValue::String(s) => YamlValue::String(Cow::Owned(s.into_owned())),
            YamlValue::Array(items) => {
                YamlValue::Array(items.into_iter().map(into_static).collect())
            }
            YamlValue::Object(m) => YamlValue::Object(
                m.into_iter()
                    .map(|(k, v)| (Cow::Owned(k.into_owned()), into_static(v)))
                    .collect(),
            ),
        },
        leading_comment: n.leading_comment.map(|c| Cow::Owned(c.into_owned())),
        inline_comment: n.inline_comment.map(|c| Cow::Owned(c.into_owned())),
    }
}
