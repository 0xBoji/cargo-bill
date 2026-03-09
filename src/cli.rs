use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "cargo", bin_name = "cargo")]
pub enum CargoCli {
    Bill(BillArgs),
}

#[derive(clap::Args, Debug)]
#[command(author, version, about = "Estimates AWS Lambda costs based on the compiled binary size.")]
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
}

pub fn parse_args() -> BillArgs {
    let CargoCli::Bill(args) = CargoCli::parse();
    args
}
