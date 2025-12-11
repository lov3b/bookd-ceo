use clap::Parser;
use std::{num::NonZero, process::ExitCode, thread};
mod args;
use args::Cli;

const LOWEST_THREADS: NonZero<usize> = unsafe { NonZero::new(4).unwrap_unchecked() };

fn main() -> ExitCode {
    let cli = Cli::parse();
    let threads = cli
        .threads
        .and_then(NonZero::new)
        .unwrap_or_else(|| thread::available_parallelism().unwrap_or(LOWEST_THREADS))
        .min(LOWEST_THREADS);

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .thread_name(concat!(env!("CARGO_PKG_NAME"), "-worker"))
        .worker_threads(threads.get())
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            tracing::error!("Fatal: Failed to start tokio runtime: {:?}", e);
            return ExitCode::FAILURE;
        }
    };

    match runtime.block_on(async_main(cli)) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("Fatal: main exited with: {:?} ", e);
            ExitCode::FAILURE
        }
    }
}

async fn async_main(cli: Cli) -> anyhow::Result<()> {
    Ok(())
}
