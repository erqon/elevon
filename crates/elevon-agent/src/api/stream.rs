use std::{convert::Infallible, time::Duration};

use axum::response::{
    Sse,
    sse::{Event, KeepAlive, KeepAliveStream},
};
use elevon_contracts::deploy::StreamEvent;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_stream::wrappers::ReceiverStream;

type StreamItem = Result<Event, Infallible>;
pub type StreamSender = Sender<StreamItem>;
pub type StreamReceiver = Receiver<StreamItem>;
pub type StreamResponse = Sse<KeepAliveStream<ReceiverStream<StreamItem>>>;

pub fn create_stream_channel() -> (StreamSender, StreamReceiver) {
    mpsc::channel::<StreamItem>(32)
}

pub async fn emit(tx: &StreamSender, event: StreamEvent) {
    let line = serde_json::to_string(&event).unwrap();
    let _ = tx.send(Ok(Event::default().data(line))).await;
}

pub async fn emit_error(tx: &StreamSender, message: String) {
    emit(tx, StreamEvent::Error { message }).await;
}

pub fn stream_response(rx: StreamReceiver) -> StreamResponse {
    let stream = ReceiverStream::new(rx);
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep alive"),
    )
}
