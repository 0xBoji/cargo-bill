use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "cargo", bin_name = "cargo")]
pub enum CargoCli {
    Bill(BillArgs),
}

#[derive(clap::Args, Debug)]
#[command(
    author,
    version,
    about = "Estimates AWS Lambda costs based on the compiled binary size."
)]
pub struct BillArgs {
    #[command(subcommand)]
    pub command: BillSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum BillSubcommands {
    /// Estimate AWS Lambda costs
    Lambda(LambdaArgs),
}

#[derive(clap::Args, Debug)]
pub struct LambdaArgs {
    /// AWS Region
    #[arg(long, default_value = "us-east-1")]
    pub region: String,

    /// Lambda Memory (MB)
    #[arg(long, default_value_t = 128)]
    pub memory: u32,

    /// Number of Executions
    #[arg(long, default_value_t = 1_000_000)]
    pub executions: u64,

    /// Architecture (x86_64 or arm64)
    #[arg(long, default_value = "x86_64")]
    pub architecture: String,

    /// Output results in JSON format
    #[arg(long)]
    pub json: bool,

    /// Include AWS Free Tier deductions (1M free requests & 400,000 GB-seconds)
    #[arg(long)]
    pub include_free_tier: bool,

    /// Assume Provisioned Concurrency (Eliminates Cold Starts)
    #[arg(long)]
    pub provisioned_concurrency: bool,
}

pub fn parse_args() -> BillArgs {
    let CargoCli::Bill(args) = CargoCli::parse();
    args
}
