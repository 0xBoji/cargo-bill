# cargo-bill

[![Crates.io](https://img.shields.io/crates/v/cargo-bill.svg)](https://crates.io/crates/cargo-bill)
[![Docs.rs](https://img.shields.io/docsrs/cargo-bill)](https://docs.rs/cargo-bill)
[![CI](https://github.com/0xBoji/cargo-bill/actions/workflows/rust.yml/badge.svg)](https://github.com/0xBoji/cargo-bill/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

`cargo-bill` is a Cargo plugin that estimates AWS Lambda cost and cold-start impact from your compiled Rust binary.
It is designed for platform and FinOps workflows where you want cost visibility before deployment.

## What it does

- Builds your project in release mode and analyzes binary footprint.
- Estimates Lambda storage and execution cost.
- Includes both Lambda cost components:
  - Compute cost (GB-seconds)
  - Request cost (`$0.20 / 1M requests`)
- Supports optional AWS Free Tier deductions:
  - `400,000 GB-seconds`
  - `1,000,000 requests`
- Fetches dynamic pricing via AWS Pricing API with fallback to static rates.
- Predicts cold-start latency from binary size and memory configuration.
- Outputs either table format or JSON for automation.

## Installation

### From crates.io

```bash
cargo install cargo-bill
```

### From install script

```bash
curl -sL https://raw.githubusercontent.com/0xBoji/cargo-bill/master/install.sh | bash
```

## Quick start

As a Cargo plugin:

```bash
cargo bill lambda
```

Equivalent binary invocation:

```bash
cargo-bill bill lambda
```

Run with explicit parameters:

```bash
cargo bill lambda \
  --region ap-southeast-2 \
  --architecture arm64 \
  --memory 512 \
  --executions 1000000 \
  --include-free-tier
```

JSON output mode:

```bash
cargo bill lambda --json
```

## CLI options

```text
--region <REGION>                     AWS region (default: us-east-1)
--memory <MEMORY_MB>                  Lambda memory in MB (default: 128)
--executions <COUNT>                  Number of invocations (default: 1,000,000)
--architecture <x86_64|arm64>         Lambda architecture (default: x86_64)
--include-free-tier                   Apply monthly free tier deductions
--provisioned-concurrency             Assume warm start (cold start = 0)
--json                                Print machine-readable JSON
```

## Cost model

Estimated execution cost is:

`total_cost = compute_cost + request_cost`

Where:

- `compute_cost = billable_gb_seconds * region_arch_price`
- `request_cost = (billable_requests / 1_000_000) * 0.20`

If `--include-free-tier` is enabled:

- `billable_gb_seconds = max(total_gb_seconds - 400000, 0)`
- `billable_requests = max(executions - 1000000, 0)`

## Accuracy notes

- Dynamic pricing uses AWS Pricing API location names mapped from AWS region codes.
- If pricing API access fails, `cargo-bill` falls back to static Lambda rates.
- On non-Linux hosts (macOS/Windows), compiled binary size may differ slightly from Amazon Linux ELF builds.
  For stricter production parity, use `cargo-lambda` or `cross` for cross-compilation.

## Example output

```text
AWS Lambda Cost Estimation Report:
+-----------------------------------------------+
| Metric                              Value      |
+===============================================+
| Binary Size (MB)                    15.52      |
| Architecture                        arm64      |
| Estimated Monthly Storage Cost      $0.0015    |
| Estimated Cost per 1000000 Requests $2.9433    |
| Predicted Cold Start Latency        1862.18 ms |
| Dynamic API Pricing Used            Yes        |
+-----------------------------------------------+
```

## CI and releases

- CI: lint + tests on push and pull request.
- Automated release PRs: `release-plz` workflow.
- Publishing: crates.io + GitHub release artifacts.

## License

MIT
