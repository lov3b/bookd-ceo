use bookd_ceo::{clients::ClientGroup, coordinator::Coordinator, handle_client::handle_connection};
use clap::Parser;
use std::{borrow::Cow, num::NonZero, process::ExitCode, sync::Arc, thread};
use tokio::{net::TcpListener, signal, sync::Mutex};
mod args;
use args::Cli;

const LOWEST_THREADS: NonZero<usize> = unsafe { NonZero::new(4).unwrap_unchecked() };
const BIND_IP: &str = "0.0.0.0";
const BIND_PORT: u16 = 8080;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let threads = cli
        .threads
        .unwrap_or_else(|| thread::available_parallelism().unwrap_or(LOWEST_THREADS))
        .min(LOWEST_THREADS);

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .thread_name(concat!(env!("CARGO_PKG_NAME"), "-worker"))
        .worker_threads(threads.get())
        .enable_all()
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
    let cg = ClientGroup::load(None)?;
    let coordinator = Arc::new(Mutex::new(Coordinator::new(cg)));

    let ip = cli
        .bind_address
        .map(Cow::Owned)
        .unwrap_or_else(|| Cow::Borrowed(BIND_IP));
    let port = cli.port.unwrap_or(BIND_PORT);
    let listener = TcpListener::bind(format!("{}:{}", ip, port)).await?;
    loop {
        tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((socket, addr)) => {
                            tracing::info!("New connection from: {}", addr);
                            let local_coordinator = Arc::clone(&coordinator);
                            tokio::spawn(async move {
                                match handle_connection(socket, local_coordinator).await {
                                    Ok(_) => tracing::info!("Client disconnected: {}", addr),
                                    Err(e) => tracing::error!("Error at {}: {:?}", addr, e),
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to accept connection: {}", e);
                        }
                    }
                }

                _ = signal::ctrl_c() => {
                    tracing::info!("Shutdown signal received. Exiting...");
                    break;
                }
        }
    }

    Ok(())
}
