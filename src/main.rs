use tokio_tungstenite::{accept_async, WebSocketStream};
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{StreamExt, SinkExt, stream::SplitSink};
use tokio::sync::mpsc;
use tokio::net::{TcpListener, TcpStream};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type Tx = tokio_tungstenite::tungstenite::protocol::Message;
type PeerMap = Arc<Mutex<HashMap<String, Connection>>>;

struct Connection {
    tx: SplitSink<WebSocketStream<TcpStream>, Message>,
    // Add other metadata here, e.g., connection status, last activity timestamp, etc.
    status: usize,
}

fn generate_unique_id() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}

async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream) {
    let ws_stream = accept_async(raw_stream).await.expect("Error during websocket handshake");
    let (tx, mut rx) = ws_stream.split();

    let connection_id = generate_unique_id();
    let connection = Connection {
        tx: tx,
        status: 0, // Initialize status or other metadata
    };

    {
        let mut peers = peer_map.lock().unwrap();
        peers.insert(connection_id.clone(), connection);
        println!("New WebSocket connection established with ID: {}", connection_id);
    }

    // Process messages
    while let Some(msg) = rx.next().await {
        match msg {
            Ok(message) => {
                println!("Received message: {:?}", message);
                // Broadcast to other peers if needed
            }
            Err(e) => eprintln!("Error: {:?}", e),
        }
    }

    // Remove connection on disconnection
    {
        let mut peers = peer_map.lock().unwrap();
        peers.remove(&connection_id);
        println!("WebSocket connection with ID: {} closed", connection_id);
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
