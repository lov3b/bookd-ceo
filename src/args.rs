use std::num::NonZero;

use clap::Parser;

#[derive(Parser)]
#[command(about = "Manages the bookd clients", long_about = None)]
pub struct Cli {
    /// At least 4 threads are required
    #[arg(short, long)]
    pub threads: Option<NonZero<usize>>,

    #[arg(short, long, default_value = "default_bind_ip")]
    /// Default is 0.0.0.0
    pub bind_address: String,

    #[arg(short, long, default_value = "default_bind_port")]
    /// Default is 8080
    pub port: u16,
}

#[allow(dead_code)]
fn default_bind_ip() -> String {
    "0.0.0.0".into()
}

#[allow(dead_code)]
fn default_bind_port() -> u16 {
    8080
}
