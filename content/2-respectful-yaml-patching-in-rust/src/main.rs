use std::{fs, path::PathBuf};

use anyhow::Context as _;
use clap::Parser;

mod backends;

#[derive(clap::Parser)]
struct Args {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Subcommand)]
enum Cmd {
    ListAssets(Op),
    DelistAssets(Op),
}

#[derive(clap::Args)]
struct Op {
    #[arg(long)]
    lib: backends::Lib,
    #[arg(short, long)]
    input: PathBuf,
    #[arg(short, long)]
    output: PathBuf,
    /// Comma-separated list of assets.
    assets: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let (op, kind) = match args.cmd {
        Cmd::ListAssets(op) => (op, OpKind::List),
        Cmd::DelistAssets(op) => (op, OpKind::Delist),
    };

    let assets: Vec<String> = op
        .assets
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let input = fs::read_to_string(&op.input).context("read input")?;
    let output = match kind {
        OpKind::List => op.lib.list_assets(&input, &assets)?,
        OpKind::Delist => op.lib.delist_assets(&input, &assets)?,
    };
    fs::write(&op.output, output).context("write output")?;
    Ok(())
}

enum OpKind {
    List,
    Delist,
}
