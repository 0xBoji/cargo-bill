mod analysis_engine;
mod builder;
mod cli;
mod pricing_provider;

use anyhow::Result;
use cli::{parse_args, BillSubcommands};
use prettytable::{format, row, Table};
use serde_json::json;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args();

    let BillSubcommands::Lambda(ref lambda_args) = args.command;
    if !lambda_args.json {
        tracing_subscriber::fmt::init();
    }

    match args.command {
        BillSubcommands::Lambda(lambda_args) => {
            if !lambda_args.json {
                info!("Initializing cargo-bill for AWS Lambda cost estimation...");
                info!(
                    "Region: {}, Memory: {} MB, Executions: {}, Architecture: {}",
                    lambda_args.region,
                    lambda_args.memory,
                    lambda_args.executions,
                    lambda_args.architecture
                );
            }

            let (binary_path, metadata) = builder::execute_build(lambda_args.json)?;

            let analysis = analysis_engine::analyze_binary(&binary_path)?;

            let stripped_str = if analysis.is_stripped { "Yes" } else { "No" };
            let costs = pricing_provider::calculate_costs(
                analysis.size_mb,
                lambda_args.executions,
                lambda_args.memory,
                &lambda_args.region,
                &lambda_args.architecture,
                lambda_args.include_free_tier,
                lambda_args.provisioned_concurrency,
            )
            .await;

            if std::env::consts::OS != "linux" && !lambda_args.json {
                warn!(
                    "You are building on '{}'. The binary size might differ slightly from the actual Amazon Linux (ELF) deployment.",
                    std::env::consts::OS
                );
                warn!("For strict production accuracy, consider cross-compiling with 'cargo-lambda' or 'cross'.");
            }

            if analysis.size_mb > 30.0 && !lambda_args.json {
                let has_heavy_deps = metadata.packages.iter().any(|p| p.name.contains("aws-sdk"));
                if has_heavy_deps {
                    warn!(
                        "Your binary is unusually large ({:.2} MB).",
                        analysis.size_mb
                    );
                    warn!("You are compiling aws-sdk crates (e.g., aws-sdk-s3 or aws-sdk-pricing) potentially with all features enabled.");
                    warn!("Consider using default-features = false to reduce Cold Start time.");
                } else {
                    warn!(
                        "Your binary is unusually large ({:.2} MB).",
                        analysis.size_mb
                    );
                    warn!("Consider optimizing your dependencies to reduce Cold Start time.");
                }
            }

            if lambda_args.json {
                let output = json!({
                    "metadata": {
                        "binary_size_mb": analysis.size_mb,
                        "architecture": lambda_args.architecture,
                        "stripped": analysis.is_stripped,
                        "has_debug_symbols": analysis.has_debug_symbols,
                        "executions": lambda_args.executions,
                        "memory_mb": lambda_args.memory,
                        "region": lambda_args.region,
                        "include_free_tier": lambda_args.include_free_tier,
                        "provisioned_concurrency": lambda_args.provisioned_concurrency
                    },
                    "estimation": costs
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
                return Ok(());
            }

            let mut table = Table::new();
            table.set_format(*format::consts::FORMAT_BORDERS_ONLY);
            table.set_titles(row!["Metric", "Value"]);
            table.add_row(row!["Binary Size (MB)", format!("{:.2}", analysis.size_mb)]);
            table.add_row(row!["Architecture", lambda_args.architecture]);
            table.add_row(row!["Stripped", stripped_str]);
            table.add_row(row![
                "Has Debug Symbols",
                if analysis.has_debug_symbols {
                    "Yes"
                } else {
                    "No"
                }
            ]);
            table.add_row(row![
                "Estimated Monthly Storage Cost",
                format!("${:.4}", costs.storage_cost_monthly)
            ]);
            table.add_row(row![
                format!("Estimated Cost per {} Requests", lambda_args.executions),
                format!("${:.4}", costs.compute_cost_1m)
            ]);
            table.add_row(row![
                "Predicted Cold Start Latency",
                format!("{:.2} ms", costs.predicted_cold_start_ms)
            ]);
            table.add_row(row![
                "Dynamic API Pricing Used",
                if costs.dynamic_pricing_used {
                    "Yes"
                } else {
                    "No (Fallback)"
                }
            ]);

            info!("\nAWS Lambda Cost Estimation Report:");
            table.printstd();
        }
    }

    Ok(())
}
