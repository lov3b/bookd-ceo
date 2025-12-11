// File: src/handle_client.rs
use crate::coordinator::Coordinator;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json as json;
use std::sync::Arc;
use thiserror::Error;
use tokio::{
    net::TcpStream,
    sync::{broadcast::error::RecvError, Mutex},
};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec, LinesCodecError}; // Use Coordinator

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON Error: {0}")]
    JsonError(#[from] json::Error),
    #[error("Codec Error: {0}")]
    CodecError(#[from] LinesCodecError),
    #[error("Client disconnected unexpectedly")]
    ClientDidntRespond,
    #[error("Broadcast Error")]
    BroadcastError,
}

#[derive(Debug, Deserialize)]
enum ServerTask {
    Ping,
    Status,
    Listen, // Client wants to subscribe to updates
}

#[derive(Debug, Deserialize)]
struct ToServer {
    task: ServerTask,
    value: json::Value,
}

#[derive(Debug, Serialize)]
enum ClientTask {
    Pong,
    StatusAnswer,
    NewAssignment,
}

#[derive(Debug, Serialize)]
struct ToClient<T> {
    task: ClientTask,
    value: T,
}

pub async fn handle_connection(
    socket: TcpStream,
    coordinator: Arc<Mutex<Coordinator>>,
) -> Result<(), ConnectionError> {
    let (reader, writer) = socket.into_split();
    let mut reader = FramedRead::new(reader, LinesCodec::new());
    let mut writer = FramedWrite::new(writer, LinesCodec::new());

    loop {
        let client_request_str = reader
            .next()
            .await
            .ok_or(ConnectionError::ClientDidntRespond)??;

        let request_data: ToServer = json::from_str(&client_request_str)?;

        match request_data.task {
            ServerTask::Ping => {
                let res = ToClient {
                    task: ClientTask::Pong,
                    value: json::Value::Null,
                };
                writer.send(json::to_string(&res)?).await?;
            }
            ServerTask::Status => {
                let count = coordinator.lock().await.get_status_count();
                let res = ToClient {
                    task: ClientTask::StatusAnswer,
                    value: count,
                };
                writer.send(json::to_string(&res)?).await?;
            }
            ServerTask::Listen => {
                tracing::info!("Client subscribed to updates.");

                let mut rx = {
                    let coord = coordinator.lock().await;
                    coord.subscribe()
                };

                loop {
                    match rx.recv().await {
                        Ok(assignment) => {
                            let notification = ToClient {
                                task: ClientTask::NewAssignment,
                                value: assignment,
                            };
                            writer.send(json::to_string(&notification)?).await?;
                        }
                        Err(RecvError::Lagged(e)) => {
                            tracing::warn!("Lag error: failed to recieve task: {:?}", e)
                        }
                        Err(_) => break,
                    }
                }

                return Ok(());
            }
        };
    }
}
