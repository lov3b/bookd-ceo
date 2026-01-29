use std::{num::NonZero, thread};

use clap::Parser;

pub const LOWEST_THREADS: NonZero<usize> = unsafe { NonZero::new(4).unwrap_unchecked() };
pub const DEFAULT_BIND_IP: &str = "0.0.0.0";
pub const DEFAULT_PORT: u16 = 8080;

#[derive(Parser)]
#[command(about = "Manages the bookd clients", long_about = None)]
pub struct Cli {
    /// At least 4 threads are required.
    /// The default is as many threads as processors.
    #[arg(short, long, default_value_t = get_default_threads())]
    pub threads: NonZero<usize>,

    #[arg(short, long, default_value_t = DEFAULT_BIND_IP.to_string())]
    pub bind_address: String,

    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    pub port: u16,
}

fn get_default_threads() -> NonZero<usize> {
    thread::available_parallelism().unwrap_or(LOWEST_THREADS)
}
