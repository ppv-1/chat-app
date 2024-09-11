use tokio_tungstenite::{accept_async, WebSocketStream};
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{StreamExt, SinkExt};
use futures_util::stream::SplitSink;
use tokio::net::{TcpListener, TcpStream};
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use tokio::io::{self, BufReader, AsyncBufReadExt};
use chrono::Local;

type PeerMap = Arc<Mutex<HashMap<String, Connection>>>;

struct Connection {
    tx: Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
    status: usize,
    username: String,
}

fn generate_unique_id() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}

fn format_message_with_timestamp( message: &Message) -> Message {
    let timestamp = Local::now().format("[%Y-%m-%d %H:%M:%S]").to_string();
    let formatted_message = match message {
        Message::Text(text) => format!("{}: {}", timestamp, text),
        Message::Close(_) => format!("{}: [Connection closed]", timestamp),
        _ => format!("{}: non-text message", timestamp),
    };
    Message::Text(formatted_message)
}

async fn broadcast_message(peer_map: &PeerMap, sender_id: &str, message: Message) {
    let peers = peer_map.lock().await;  // Await the lock

    for (id, peer) in peers.iter() {
        if id != sender_id {
            let tx = peer.tx.clone();  // Clone Arc to share ownership
            let mut tx = tx.lock().await;
            let timestamped_message = format_message_with_timestamp(&message);
            if let Err(e) = tx.send(timestamped_message).await {
                eprintln!("Failed to send message to {}: {:?}", id, e);
            }
        }
    }
}

async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream) {
    let ws_stream = accept_async(raw_stream).await.expect("Error during websocket handshake");
    let (tx, mut rx) = ws_stream.split();

    let connection_id = generate_unique_id();
    let username = if let Some(Ok(Message::Text(name))) = rx.next().await {
        name
    } else {
        "Anonymous".to_string() // Default to "Anonymous" if username is not provided
    };

    let connection = Connection {
        tx: Arc::new(Mutex::new(tx)),
        status: 0,
        username: username.clone(),
    };

    {
        let mut peers = peer_map.lock().await;  // Await the lock
        peers.insert(connection_id.clone(), connection);
        println!("New WebSocket connection established with ID: {} and Username: {}", connection_id, username);
    }

    // Process messages
    while let Some(msg) = rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                println!(" {} {}", username, text);
                broadcast_message(&peer_map, &connection_id, Message::Text(text)).await;
            }
            
            Ok(message) => {
                println!(" {}: {:?}", username, message);
                broadcast_message(&peer_map, &connection_id, message).await;
            }
            Err(e) => eprintln!("Error: {:?}", e),
        }
    }

    // Remove connection on disconnection
    {
        let mut peers = peer_map.lock().await;  // Await the lock
        peers.remove(&connection_id);
        println!("WebSocket connection with ID: {} closed with {}", connection_id, username);
    }
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");

    println!("Chat server running on {}", addr);

    let peer_map: PeerMap = Arc::new(Mutex::new(HashMap::new()));

    // Spawn a task to handle terminal input
    let peer_map_for_input = peer_map.clone();
    tokio::spawn(async move {
        let mut stdin = BufReader::new(io::stdin());
        let mut line = String::new();
        while let Ok(bytes) = stdin.read_line(&mut line).await {
            if bytes == 0 {
                break; // End of input
            }
            let msg = line.trim().to_string();
            if msg == "close" {
                println!("Closing server...");
                // Handle server shutdown logic here
                break;
            } else {
                // Broadcast the message to all connected peers
                let message = Message::Text(msg);
                broadcast_message(&peer_map_for_input, "", message).await;
            }
            line.clear(); // Clear the buffer for the next line
        }
    });

    // Accept and handle WebSocket connections
    while let Ok((stream, _)) = listener.accept().await {
        let peer_map = peer_map.clone();
        tokio::spawn(async move {
            handle_connection(peer_map, stream).await;
        });
    }
}
