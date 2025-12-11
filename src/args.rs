use clap::Parser;

#[derive(Parser)]
#[command(about = "Manages the bookd clients", long_about = None)]
pub struct Cli {
    /// At least 4 threads are required
    #[arg(short, long)]
    pub threads: Option<usize>,

    #[arg(short, long)]
    pub bind_address: Option<String>,

    #[arg(short, long)]
    pub port: Option<u16>,
}
