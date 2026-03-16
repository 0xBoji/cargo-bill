use anyhow::{Result};
use cargo_metadata::Metadata;
use serde::Serialize;
use std::fs;
use toml::Value;

#[derive(Serialize, Debug, Clone)]
pub struct OptimizationTip {
    pub category: String,
    pub message: String,
    pub fix_suggestion: String,
    pub impact: String,
}

pub fn check_optimizations(metadata: &Metadata) -> Vec<OptimizationTip> {
    let mut tips = Vec::new();

    // 1. Check for panic = "abort"
    if let Ok(manifest) = read_manifest(metadata) {
        if !has_panic_abort(&manifest) {
            tips.push(OptimizationTip {
                category: "Binary Size".to_string(),
                message: "Panic strategy is not set to 'abort' in release profile.".to_string(),
                fix_suggestion: "[profile.release]\npanic = \"abort\"".to_string(),
                impact: "Reduces binary size by removing unwinding code.".to_string(),
            });
        }

        if !has_lto_true(&manifest) {
            tips.push(OptimizationTip {
                category: "Binary Size".to_string(),
                message: "Link Time Optimization (LTO) is not enabled in release profile.".to_string(),
                fix_suggestion: "[profile.release]\nlto = true".to_string(),
                impact: "Allows the compiler to optimize across crate boundaries, significantly reducing binary size.".to_string(),
            });
        }

        if !has_codegen_units_1(&manifest) {
            tips.push(OptimizationTip {
                category: "Binary Size".to_string(),
                message: "Codegen units is not set to 1 in release profile.".to_string(),
                fix_suggestion: "[profile.release]\ncodegen-units = 1".to_string(),
                impact: "Improves optimization and reduces binary size, though increases compilation time.".to_string(),
            });
        }
    }

    tips
}

fn read_manifest(metadata: &Metadata) -> Result<Value> {
    let manifest_path = metadata.workspace_root.join("Cargo.toml");
    let content = fs::read_to_string(manifest_path)?;
    let value = content.parse::<Value>()?;
    Ok(value)
}

fn has_panic_abort(manifest: &Value) -> bool {
    manifest
        .get("profile")
        .and_then(|p| p.get("release"))
        .and_then(|r| r.get("panic"))
        .and_then(|panic| panic.as_str())
        == Some("abort")
}

fn has_lto_true(manifest: &Value) -> bool {
    let lto = manifest
        .get("profile")
        .and_then(|p| p.get("release"))
        .and_then(|r| r.get("lto"));
    
    match lto {
        Some(Value::Boolean(b)) => *b,
        Some(Value::String(s)) => s == "true" || s == "fat" || s == "thin",
        _ => false,
    }
}

fn has_codegen_units_1(manifest: &Value) -> bool {
    manifest
        .get("profile")
        .and_then(|p| p.get("release"))
        .and_then(|r| r.get("codegen-units"))
        .and_then(|c| c.as_integer())
        == Some(1)
}
