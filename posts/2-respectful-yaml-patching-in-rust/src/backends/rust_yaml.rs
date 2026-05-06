use anyhow::Context as _;
use rust_yaml::{LoaderType, Value, Yaml, YamlConfig};

const ROOT: &str = "asset_groups";
const DEFAULT: &str = "default";

fn yaml() -> Yaml {
    let mut cfg = YamlConfig::default();
    cfg.loader_type = LoaderType::RoundTrip;
    cfg.preserve_comments = true;
    Yaml::with_config(cfg)
}

fn key(s: &str) -> Value {
    Value::String(s.to_string())
}

pub fn list_assets(input: &str, assets: &[String]) -> anyhow::Result<String> {
    let yml = yaml();
    let mut commented = yml.load_str_with_comments(input).map_err(to_anyhow)?;
    let root = root_mut(&mut commented.value)?;

    let existing: std::collections::HashSet<String> = root
        .iter()
        .filter_map(|(_, v)| v.as_sequence())
        .flat_map(|seq| seq.iter().filter_map(|v| v.as_str().map(str::to_string)))
        .collect();

    let to_add: Vec<&String> = assets.iter().filter(|a| !existing.contains(*a)).collect();
    if to_add.is_empty() {
        return yml.dump_str_with_comments(&commented).map_err(to_anyhow);
    }

    let default_seq = root
        .get_mut(&key(DEFAULT))
        .context("no `default` group")?
        .as_sequence_mut()
        .context("`default` is not a sequence")?;
    for asset in to_add {
        default_seq.push(Value::String(asset.clone()));
    }
    default_seq.sort_by(|a, b| a.as_str().unwrap_or("").cmp(b.as_str().unwrap_or("")));

    yml.dump_str_with_comments(&commented).map_err(to_anyhow)
}

pub fn delist_assets(input: &str, assets: &[String]) -> anyhow::Result<String> {
    let yml = yaml();
    let mut commented = yml.load_str_with_comments(input).map_err(to_anyhow)?;
    let root = root_mut(&mut commented.value)?;

    let to_remove: std::collections::HashSet<&str> =
        assets.iter().map(String::as_str).collect();

    let group_keys: Vec<Value> = root.keys().cloned().collect();
    let mut empty_groups: Vec<Value> = Vec::new();
    for k in &group_keys {
        let Some(seq) = root.get_mut(k).and_then(|v| v.as_sequence_mut()) else {
            continue;
        };
        seq.retain(|v| match v.as_str() {
            Some(s) => !to_remove.contains(s),
            None => true,
        });
        if seq.is_empty() {
            empty_groups.push(k.clone());
        }
    }
    for k in empty_groups {
        root.shift_remove(&k);
    }

    yml.dump_str_with_comments(&commented).map_err(to_anyhow)
}

fn root_mut(value: &mut Value) -> anyhow::Result<&mut indexmap::IndexMap<Value, Value>> {
    let map = value
        .as_mapping_mut()
        .context("root is not a mapping")?;
    let groups = map
        .get_mut(&key(ROOT))
        .context("no `asset_groups` key")?
        .as_mapping_mut()
        .context("`asset_groups` is not a mapping")?;
    Ok(groups)
}

fn to_anyhow(e: rust_yaml::Error) -> anyhow::Error {
    anyhow::anyhow!("{e}")
}
