use std::{error::Error, path::PathBuf};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    event: String,
    #[serde(default)]
    data: serde_json::Value,
}

#[derive(Parser)]
#[command(about = "Unix domain socket JSON demo across two processes")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Process A: accept connections and read JSON`
    Listen {
        #[arg(long, short)]
        socket: PathBuf,
    },
    /// Process B: connect and write a JSON message
    Send {
        #[arg(long, short)]
        socket: PathBuf,
        /// Event name, e.g. ping
        #[arg(long, short, default_value = "ping")]
        event: String,
        /// JSON value for the data field, e.g. '{"hello":"world"}'
        #[arg(long, short, default_value = "{}")]
        data: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    match Cli::parse().command {
        Command::Listen { socket } => listen(socket).await?,
        Command::Send {
            socket,
            event,
            data,
        } => send(socket, event, data).await?,
    }

    Ok(())
}

async fn listen(socket: PathBuf) -> Result<(), Box<dyn Error>> {
    if socket.exists() {
        std::fs::remove_file(&socket)?;
    }

    let listener = UnixListener::bind(&socket)?;
    tracing::info!(path = %socket.display(), "listen process: waiting for connections");

    loop {
        let (mut stream, _) = listener.accept().await?;
        tracing::info!("listen process: accepted, reading…");

        let mut buf = vec![];
        stream.read_to_end(&mut buf).await?;

        match serde_json::from_slice::<Message>(&buf) {
            Ok(message) => {
                tracing::info!(
                    event = %message.event,
                    data = %message.data,
                    "listen process received json"
                );
            }
            Err(err) => {
                let raw = String::from_utf8_lossy(&buf);
                tracing::error!(%err, %raw, "listen process: invalid json");
            }
        }

        tracing::info!("listen process: waiting for next connection");
    }
}

async fn send(socket: PathBuf, event: String, data: String) -> Result<(), Box<dyn Error>> {
    let data: serde_json::Value = serde_json::from_str(&data)?;
    let message = Message { event, data };
    let payload = serde_json::to_vec(&message)?;

    tracing::info!(
        path = %socket.display(),
        json = %serde_json::to_string(&message)?,
        "send process: connecting"
    );

    let mut stream = UnixStream::connect(&socket).await?;
    stream.write_all(&payload).await?;
    stream.shutdown().await?;
    tracing::info!("send process: done");

    Ok(())
}
