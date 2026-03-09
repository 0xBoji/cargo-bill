const S3_GB_MONTHLY: f64 = 0.10;
const LAMBDA_GB_SECOND_X86: f64 = 0.0000166667;
const LAMBDA_GB_SECOND_ARM64: f64 = 0.0000133334;

use anyhow::{anyhow, Context, Result};

use aws_sdk_pricing::types::Filter;
use aws_sdk_pricing::types::FilterType;
use aws_sdk_pricing::Client as PricingClient;
use serde_json::Value;

pub struct PricingEstimate {
    pub storage_cost_monthly: f64,
    pub compute_cost_1m: f64,
    pub predicted_cold_start_ms: f64,
    pub dynamic_pricing_used: bool,
}

fn get_location_name(region: &str) -> &str {
    match region {
        "us-east-1" => "US East (N. Virginia)",
        "us-east-2" => "US East (Ohio)",
        "us-west-1" => "US West (N. California)",
        "eu-west-1" => "EU (Ireland)",
        "eu-central-1" => "EU (Frankfurt)",
        "ap-southeast-1" => "Asia Pacific (Singapore)",
        // Add more common regions...
        _ => "US East (N. Virginia)", // Fallback default
    }
}

pub async fn fetch_real_lambda_price(region: &str, architecture: &str) -> Result<f64> {
    // AWS Pricing API ONLY exists in us-east-1 and ap-south-1.
    // Force SDK to us-east-1 regardless of the user's workload region.
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .load()
        .await;
    let client = PricingClient::new(&config);

    let location_name = get_location_name(region);

    let location_filter = Filter::builder()
        .field("Location")
        .r#type(FilterType::TermMatch)
        .value(location_name)
        .build()
        .map_err(|e| anyhow!("Failed to build location filter: {}", e))?;

    // We will pull the list of products for the Location, and then manually filter the JSON for the architecture
    // This is safer than relying on erratic AWS Pricing API filter keys for architecture

    let resp = client
        .get_products()
        .service_code("AWSLambda")
        .filters(location_filter)
        .send()
        .await
        .context("AWS Pricing API call failed")?;

    let price_list = resp.price_list();
    if price_list.is_empty() {
        return Err(anyhow!("Price list empty"));
    }

    let is_arm64 = architecture == "arm64";

    let mut found_price: Option<f64> = None;

    // Loop through the price lists to manually filter for architecture
    for json_str in price_list {
        if let Ok(v) = serde_json::from_str::<Value>(json_str) {
            // Check if computing architecture matches
            // It could be in product.attributes.usagetype (e.g., EUC1-ARM-Lambda-GB-Second)
            // or product.attributes.processorArchitecture (e.g., ARM64)
            let mut matches_arch = false;

            if let Some(attributes) = v.get("product").and_then(|p| p.get("attributes")) {
                let usage_type = attributes
                    .get("usagetype")
                    .and_then(|u| u.as_str())
                    .unwrap_or("");
                let proc_arch = attributes
                    .get("processorArchitecture")
                    .and_then(|a| a.as_str())
                    .unwrap_or("");

                // Exclude Edge / Provisioned / etc
                if !usage_type.contains("Lambda-GB-Second") {
                    continue;
                }

                if is_arm64 {
                    if usage_type.contains("ARM") || proc_arch.to_uppercase() == "ARM64" {
                        matches_arch = true;
                    }
                } else if !usage_type.contains("ARM")
                    && (proc_arch == "x86_64" || proc_arch == "AMD64" || proc_arch.is_empty())
                {
                    matches_arch = true;
                }
            }

            if !matches_arch {
                continue;
            }

            // Parse price
            if let Some(terms) = v.get("terms").and_then(|t| t.get("OnDemand")) {
                if let Some(on_demand_obj) = terms.as_object() {
                    if let Some(first_offer) = on_demand_obj.values().next() {
                        if let Some(dimensions) = first_offer
                            .get("priceDimensions")
                            .and_then(|d| d.as_object())
                        {
                            if let Some(first_dimension) = dimensions.values().next() {
                                if let Some(price_str) = first_dimension
                                    .get("pricePerUnit")
                                    .and_then(|p| p.get("USD"))
                                    .and_then(|u| u.as_str())
                                {
                                    if let Ok(price) = price_str.parse::<f64>() {
                                        found_price = Some(price);
                                        break; // Found the matching architecture price!
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    found_price.ok_or_else(|| {
        anyhow!(
            "Could not find pricing data for architecture: {}",
            architecture
        )
    })
}

pub fn predict_cold_start(size_mb: f64, memory_mb: u32) -> f64 {
    let base_latency_per_mb = 15.0; // Assume 15ms per MB on lowest tier

    // Memory scaling factor: 1024MB is our baseline (factor = 1.0)
    // If you use 128MB, it's 8x slower to load into memory.
    let memory_factor = 1024.0 / (memory_mb as f64);

    // Calculate the weighted cold start
    let cold_start_ms = (size_mb * base_latency_per_mb) * memory_factor;

    // Rust is incredibly fast, so cap the minimum latency
    cold_start_ms.max(20.0)
}

pub async fn calculate_costs(
    size_mb: f64,
    executions: u64,
    memory_mb: u32,
    region: &str,
    architecture: &str,
) -> PricingEstimate {
    let size_gb = size_mb / 1024.0;
    let storage_cost_monthly = size_gb * S3_GB_MONTHLY;

    let cold_start_ms = predict_cold_start(size_mb, memory_mb);
    let baseline_duration_ms = 100.0;

    let mem_gb = memory_mb as f64 / 1024.0;
    let total_duration_seconds = (baseline_duration_ms + cold_start_ms) / 1000.0;

    // Try finding dynamic rate
    let (lambda_gb_second, dynamic_pricing_used) =
        match fetch_real_lambda_price(region, architecture).await {
            Ok(price) => (price, true),
            Err(_) => {
                let base_rate = if architecture == "arm64" {
                    LAMBDA_GB_SECOND_ARM64
                } else {
                    LAMBDA_GB_SECOND_X86
                };
                (base_rate, false)
            }
        };

    let cost_per_execution = (mem_gb * total_duration_seconds) * lambda_gb_second;
    let compute_cost_1m = cost_per_execution * (executions as f64);

    PricingEstimate {
        storage_cost_monthly,
        compute_cost_1m,
        predicted_cold_start_ms: cold_start_ms,
        dynamic_pricing_used,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predict_cold_start() {
        // Binary 10MB, RAM 1024MB -> Bằng đúng base latency
        assert_eq!(predict_cold_start(10.0, 1024), 150.0);

        // Binary 10MB, RAM 128MB -> Chậm gấp 8 lần
        assert_eq!(predict_cold_start(10.0, 128), 1200.0);

        // Binary rất nhỏ, bị chặn ở mức min 20ms
        assert_eq!(predict_cold_start(0.5, 1024), 20.0);
    }
}
