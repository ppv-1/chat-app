use tokio_tungstenite::{accept_async, WebSocketStream};
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{StreamExt, SinkExt};
use futures_util::stream::SplitSink;
use tokio::net::{TcpListener, TcpStream};
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use chrono::Local;

type PeerMap = Arc<Mutex<HashMap<String, Connection>>>;

struct Connection {
    tx: Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
    username: String,
}

fn generate_unique_id() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}

async fn send_private_message(peer_map: &PeerMap, sender: &str, recipient: &str, message: &str) {
    let peers = peer_map.lock().await;
    if let Some(peer) = peers.values().find(|p| p.username == recipient) {
        let tx = peer.tx.clone();
        let mut tx = tx.lock().await;
        let private_message = Message::Text(format!("(Private) {} -> {}: {}", sender, recipient, message));
        if let Err(e) = tx.send(private_message).await {
            eprintln!("Failed to send private message to {}: {:?}", recipient, e);
        }
    } else {
        eprintln!("User {} not found", recipient);
    }
}

async fn broadcast_message(peer_map: &PeerMap, sender_id: &str, message: Message) {
    let peers = peer_map.lock().await;
    for (id, peer) in peers.iter() {
        if id != sender_id {
            let tx = peer.tx.clone();
            let mut tx = tx.lock().await;
            if let Err(e) = tx.send(message.clone()).await {
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
        "Anonymous".to_string()
    };

    let connection = Connection {
        tx: Arc::new(Mutex::new(tx)),
        username: username.clone(),
    };

    {
        let mut peers = peer_map.lock().await;
        peers.insert(connection_id.clone(), connection);
        println!("New WebSocket connection established with ID: {} and Username: {}", connection_id, username);
    }

    while let Some(msg) = rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if text.starts_with("/whisper ") {
                    // Handle whispers (e.g., /whisper username message)
                    let parts: Vec<&str> = text.splitn(3, ' ').collect();
                    if parts.len() == 3 {
                        let recipient = parts[1];  // Extract the recipient's username
                        let private_message = parts[2];  // Extract the private message content
                        send_private_message(&peer_map, &username, recipient, private_message).await;
                    } else {
                        eprintln!("Invalid whisper command format. Expected: /whisper <username> <message>");
                    }
                } else {
                    // Broadcast normal message
                    println!("{} {}", username, text);
                    broadcast_message(&peer_map, &connection_id, Message::Text(text)).await;
                }
            }
            Ok(message) => {
                broadcast_message(&peer_map, &connection_id, message).await;
            }
            Err(e) => eprintln!("Error: {:?}", e),
        }
    }

    {
        let mut peers = peer_map.lock().await;
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

    while let Ok((stream, _)) = listener.accept().await {
        let peer_map = peer_map.clone();
        tokio::spawn(async move {
            handle_connection(peer_map, stream).await;
        });
    }
}
