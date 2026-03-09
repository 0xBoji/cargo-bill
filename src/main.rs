mod analysis_engine;
mod builder;
mod cli;
mod pricing_provider;

use anyhow::Result;
use cli::{parse_args, BillSubcommands};
use prettytable::{format, row, Table};

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args();

    match args.command {
        BillSubcommands::Lambda(lambda_args) => {
            println!("Initializing cargo-bill for AWS Lambda cost estimation...");
            println!(
                "Region: {}, Memory: {} MB, Executions: {}, Architecture: {}",
                lambda_args.region,
                lambda_args.memory,
                lambda_args.executions,
                lambda_args.architecture
            );

            let (binary_path, metadata) = builder::execute_build()?;

            let analysis = analysis_engine::analyze_binary(&binary_path)?;

            let stripped_str = if analysis.is_stripped { "Yes" } else { "No" };
            let costs = pricing_provider::calculate_costs(
                analysis.size_mb,
                lambda_args.executions,
                lambda_args.memory,
                &lambda_args.region,
                &lambda_args.architecture,
            )
            .await;

            if analysis.size_mb > 30.0 {
                let has_heavy_deps = metadata.packages.iter().any(|p| p.name.contains("aws-sdk"));
                if has_heavy_deps {
                    println!(
                        "\nWarning: Your binary is unusually large ({:.2} MB).",
                        analysis.size_mb
                    );
                    println!("You are compiling aws-sdk crates (e.g., aws-sdk-s3 or aws-sdk-pricing) potentially with all features enabled.");
                    println!("Consider using default-features = false to reduce Cold Start time.");
                } else {
                    println!(
                        "\nWarning: Your binary is unusually large ({:.2} MB).",
                        analysis.size_mb
                    );
                    println!("Consider optimizing your dependencies to reduce Cold Start time.");
                }
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

            println!("\nAWS Lambda Cost Estimation Report:");
            table.printstd();
        }
    }

    Ok(())
}
