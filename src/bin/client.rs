use tokio_tungstenite::connect_async;
use futures_util::{StreamExt, SinkExt};
use tokio::io::{self, BufReader, AsyncBufReadExt};
use tokio_tungstenite::tungstenite::protocol::Message;
use chrono::Local;

#[tokio::main]
async fn main() {
    // let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel();

    // Connect to WebSocket and split into `write` and `read`
    let url = url::Url::parse("ws://127.0.0.1:8080").unwrap();
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // Create an async stdin reader using tokio's async system
    let mut stdin = BufReader::new(io::stdin());

    // Prompt user for a username
    println!("Enter your username:");
    let mut username = String::new();
    stdin.read_line(&mut username).await.expect("Failed to read username");
    let username = username.trim();

    write.send(Message::Text(username.to_string())).await.unwrap();
    

    fn format_message_with_timestamp(message: &str) -> Message {
        let timestamp = Local::now().format("[%Y-%m-%d %H:%M:%S]").to_string();
        let formatted_message = format!("{}: {}", timestamp, message);
        Message::Text(formatted_message)
    }

    // Task to read from stdin and send it to WebSocket
    tokio::spawn(async move {
        let mut line = String::new();

        loop {
            // Read input from stdin
            let bytes = stdin.read_line(&mut line).await.expect("Failed to read line");
            if bytes == 0 {
                break; // End of input
            }
            let msg = line.trim().to_string();
            if msg == "close" {
                write.send(Message::Close(None)).await.unwrap();
                break;
            }
            let timestamped_msg = format_message_with_timestamp( &msg);
            write.send(timestamped_msg).await.unwrap();
            line.clear(); // Clear the buffer for the next line
        }
    });


    // Receive messages from the WebSocket server
    while let Some(Ok(msg)) = read.next().await {
        match msg {
            // Handle text messages
            Message::Text(text) => println!("Server {}", text),
    
            Message::Close(_) => {
                println!("Connection closed");
                break; // Exit the loop if the server closes the connection
            }
            _ => println!("Received non-text message: {:?}", msg),
        }
    }
}
