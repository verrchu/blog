use std::collections::HashSet;

use anyhow::Context as _;
use clap::Parser;
use yamlpatch::{Op, Patch, apply_yaml_patches};

const INPUT: &str = "\
asset_groups:
  default:
    - 1INCH
    - ATOM
    - LINK
";

#[derive(Parser)]
struct Args {
    /// Comma-separated assets to list (i.e. add to `default` if missing).
    #[arg(long)]
    assets: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let new_assets: Vec<String> = args
        .assets
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    // parse old assets
    let parsed: serde_yaml::Value = serde_yaml::from_str(INPUT).unwrap();
    let default_old = parsed
        .get("asset_groups")
        .and_then(|v| v.get("default"))
        .and_then(|v| v.as_sequence())
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();

    // construct new assets
    let mut default_new = Vec::<String>::from_iter(HashSet::<String>::from_iter(
        default_old.iter().cloned().chain(new_assets),
    ));
    default_new.sort();

    let mut patches: Vec<Patch> = Vec::new();
    for item in default_new {
        patches.push(Patch {
            route: yamlpath::route!("asset_groups", "default"),
            operation: Op::Append {
                value: serde_yaml::Value::String(item),
            },
        });
    }

    for _ in 0..default_old.len() {
        patches.push(Patch {
            route: yamlpath::route!("asset_groups", "default", 0usize),
            operation: Op::Remove,
        });
    }

    let doc = yamlpath::Document::new(INPUT.to_string()).unwrap();
    let new_doc = apply_yaml_patches(&doc, &patches).context("apply patches")?;
    print!("{}", new_doc.source());
    Ok(())
}
