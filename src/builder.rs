use anyhow::{anyhow, Context, Result};
use std::env;
use std::path::PathBuf;
use std::process::Command;

use tracing::info;

pub fn execute_build(mute_build_output: bool) -> Result<(PathBuf, cargo_metadata::Metadata)> {
    if !mute_build_output {
        info!("Executing `cargo build --release`...");
    }

    let status = Command::new("cargo")
        .args(["build", "--release"])
        .status()
        .context("Failed to execute cargo build")?;

    if !status.success() {
        return Err(anyhow!("Cargo build failed"));
    }

    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .context("Failed to get cargo metadata")?;

    let root_package = metadata
        .root_package()
        .context("Could not find root package in cargo metadata")?;

    let target_dir = &metadata.target_directory;

    let bin_name = root_package
        .targets
        .iter()
        .find(|t| t.kind.iter().any(|k| k == "bin"))
        .map(|t| t.name.clone())
        .context("No binary target found in package")?;

    let mut binary_path = target_dir.clone().into_std_path_buf();
    binary_path.push("release");
    binary_path.push(&bin_name);

    // basic extension resolution for Windows as requested for completeness
    if env::consts::OS == "windows" {
        binary_path.set_extension("exe");
    }

    if binary_path.exists() {
        Ok((binary_path, metadata))
    } else {
        Err(anyhow!(
            "Binary not found at expected path: {:?}",
            binary_path
        ))
    }
}
