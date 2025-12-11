use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json as json;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec, LinesCodecError};

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
}

#[derive(Debug, Deserialize)]
enum ServerTask {
    Ping,
    Status,
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
    Schedule,
}

#[derive(Debug, Serialize)]
struct ToClient<T> {
    task: ClientTask,
    value: T,
}

pub async fn handle_connection(socket: TcpStream) -> Result<(), ConnectionError> {
    let (reader, writer) = socket.into_split();

    let mut framed_reader = FramedRead::new(reader, LinesCodec::new());
    let mut framed_writer = FramedWrite::new(writer, LinesCodec::new());

    loop {
        let client_request_str = framed_reader
            .next()
            .await
            .ok_or(ConnectionError::ClientDidntRespond)??;

        let request_data: ToServer = json::from_str(&client_request_str)?;
        let response = match request_data.task {
            ServerTask::Ping => ToClient {
                task: ClientTask::Pong,
                value: json::Value::Null,
            },
            ServerTask::Status => ToClient {
                task: ClientTask::StatusAnswer,
                value: json::json!(1),
            },
        };

        let response_str = json::to_string(&response)?;
        framed_writer.send(response_str).await?;
    }
}
