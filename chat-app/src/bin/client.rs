use tokio_tungstenite::connect_async;
use futures_util::{StreamExt, SinkExt};
use tokio::io::{self, BufReader, AsyncBufReadExt};
use tokio::sync::mpsc;
use tokio::sync::Mutex; // Use tokio's async Mutex
use tokio_tungstenite::tungstenite::protocol::Message;
use std::sync::Arc; // Use Arc to share ownership

#[tokio::main]
async fn main() {
    let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel();

    // Connect to WebSocket and split into `write` and `read`
    let url = url::Url::parse("ws://127.0.0.1:8080").unwrap();
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    let (write, mut read) = ws_stream.split();

    // Wrap `write` in `Arc<Mutex<>>`
    let write = Arc::new(Mutex::new(write));
    let write_clone = write.clone();

    // Task to read from stdin and send it to WebSocket
    tokio::spawn(async move {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        while let Ok(bytes) = reader.read_line(&mut line).await {
            if bytes == 0 {
                break; // End of input
            }
            stdin_tx.send(line.trim().to_string()).unwrap(); // Send the trimmed line to the channel
            line.clear(); // Clear the buffer for the next line
        }
    });

    // Task to read from stdin channel and send to WebSocket server
    tokio::spawn(async move {
        while let Some(msg) = stdin_rx.recv().await {
            if msg.trim() == "close" {
                let mut write = write_clone.lock().await; // Lock the mutex to access `write`
                write.send(Message::Close(None)).await.unwrap();
                break;
            }
            let mut write = write_clone.lock().await; // Lock the mutex to access `write`
            write.send(msg.into()).await.unwrap();
        }
    });

    // Receive messages from the WebSocket server
    while let Some(Ok(msg)) = read.next().await {
        println!("Received: {}", msg.to_text().unwrap());
    }
}
