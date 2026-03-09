use anyhow::{Context, Result};
use object::{File, Object};
use std::fs;
use std::path::Path;

pub struct BinaryAnalysis {
    pub size_mb: f64,
    pub is_stripped: bool,
    pub has_debug_symbols: bool,
}

pub fn analyze_binary(path: &Path) -> Result<BinaryAnalysis> {
    let metadata = fs::metadata(path).context("Failed to read binary metadata")?;
    let size_bytes = metadata.len();
    let size_mb = size_bytes as f64 / (1024.0 * 1024.0);

    let data = fs::read(path).context("Failed to read binary file")?;

    let object_file = File::parse(&*data).context("Failed to parse object file")?;

    let has_debug_symbols = object_file.has_debug_symbols();
    let is_stripped = !has_debug_symbols && object_file.symbols().count() == 0;

    Ok(BinaryAnalysis {
        size_mb,
        is_stripped,
        has_debug_symbols,
    })
}
