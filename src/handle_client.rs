use crate::coordinator::{BroadcastEvent, Coordinator};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json as json;
use std::{str::FromStr, sync::Arc};
use thiserror::Error;
use tokio::{
    net::TcpStream,
    sync::{broadcast::error::RecvError, Mutex},
};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec, LinesCodecError};
use uuid::Uuid;

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
    #[error("Client failed to identify itself: {0}")]
    InvalidIdentification(String),
    #[error("Client didn't identify itself ip: {0}")]
    NoIdentification(String),
    #[error("Broadcast Error")]
    BroadcastError,
}

#[derive(Debug, Deserialize)]
enum Action {
    Identify,
    Ping,
    Status,
    Listen,
    CurrentBooking,
}

#[derive(Debug, Deserialize)]
struct Request {
    action: Action,
    value: json::Value,
}

#[derive(Debug, Serialize)]
enum ClientAction {
    Pong,
    StatusAnswer,
    NewAssignment,
    Cancellation,
}

#[derive(Debug, Serialize)]
struct Response<T> {
    action: ClientAction,
    value: T,
}

pub async fn handle_connection(
    socket: TcpStream,
    coordinator: Arc<Mutex<Coordinator>>,
) -> Result<(), ConnectionError> {
    let address = socket
        .peer_addr()
        .map(|address| address.to_string())
        .unwrap_or_else(|_| "?".into());

    let (reader, writer) = socket.into_split();
    let mut reader = FramedRead::new(reader, LinesCodec::new());
    let mut writer = FramedWrite::new(writer, LinesCodec::new());
    let mut client_uuid = None;

    loop {
        let client_request_str = reader
            .next()
            .await
            .ok_or(ConnectionError::ClientDidntRespond)??;

        let request_data: Request = json::from_str(&client_request_str)?;

        match (client_uuid.as_ref(), request_data.action) {
            (_, Action::Identify) => {
                let uuid_field = request_data.value.as_str().ok_or_else(|| {
                    ConnectionError::InvalidIdentification(format!(
                        "No uuid field was supplied by client@{}",
                        address
                    ))
                })?;
                let uuid = Uuid::from_str(uuid_field).map_err(|e| {
                    ConnectionError::InvalidIdentification(format!(
                        "Failed to parse uuid({}) from client@{}: {:?}",
                        uuid_field, address, e
                    ))
                })?;

                let is_valid = coordinator.lock().await.client_identified(uuid).await;
                if !is_valid {
                    return Err(ConnectionError::InvalidIdentification(format!(
                        "Client@{} sent an unregistered uuid({})",
                        address, uuid
                    )));
                }
                client_uuid = Some(uuid);
            }
            (None, _) => return Err(ConnectionError::NoIdentification(address)),
            (Some(_), Action::Ping) => {
                writer
                    .send(json::to_string(&Response {
                        action: ClientAction::Pong,
                        value: json::Value::Null,
                    })?)
                    .await?;
                tracing::info!("Sent Pong to client@{}", address);
            }
            (Some(_), Action::Status) => {
                let count = coordinator.lock().await.get_status_count();
                writer
                    .send(json::to_string(&Response {
                        action: ClientAction::StatusAnswer,
                        value: count,
                    })?)
                    .await?;
                tracing::info!("Sent status of {} to client@{}", count, address)
            }
            (Some(uuid), Action::CurrentBooking) => {
                let current_state = coordinator.lock().await.get_assignments(uuid).await;

                writer
                    .send(json::to_string(&Response {
                        action: ClientAction::NewAssignment,
                        value: current_state,
                    })?)
                    .await?;
            }
            (Some(_), Action::Listen) => {
                tracing::info!("Client@{} subscribed to updates.", address);

                let mut rx = {
                    let coord = coordinator.lock().await;
                    coord.subscribe()
                };

                loop {
                    match rx.recv().await {
                        Ok(event) => match event {
                            BroadcastEvent::Assigned(assignment) => {
                                let notification = Response {
                                    action: ClientAction::NewAssignment,
                                    value: assignment,
                                };
                                writer.send(json::to_string(&notification)?).await?;
                            }
                            BroadcastEvent::Cancelled(booking_id) => {
                                let notification = Response {
                                    action: ClientAction::Cancellation,
                                    value: booking_id,
                                };
                                writer.send(json::to_string(&notification)?).await?;
                            }
                        },
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
