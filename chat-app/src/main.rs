use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

type Tx = tokio::sync::mpsc::UnboundedSender<String>;
type PeerMap = Arc<Mutex<HashMap<String, Tx>>>;

async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream) {
    let ws_stream = accept_async(raw_stream).await.expect("Error during websocket handshake");
    let (mut write, mut read) = ws_stream.split();
    let mut peers = peer_map.lock().unwrap();
    let connection_id = generate_unique_id(); 
    peers.insert(connection_id, (write, 0)); // Storing connection metadata
    
    println!("New WebSocket connection established with ID: {}", connection_id);



    // Broadcast messages to all peers
    while let Some(msg) = read.next().await {
        let msg = match msg {
            Ok(msg) => match msg.to_text() {
                Ok(text) => text.to_string(), // Convert to String immediately
                Err(e) => {
                    eprintln!("Error converting message to text: {}", e);
                    continue;
                }
            },
            Err(e) => {
                eprintln!("Error reading message: {}", e);
                continue;
            }
        };
        println!("Received message: {}", msg);

        // Broadcast the message to other clients
        let peers = peer_map.lock().unwrap();
        for (peer, tx) in peers.iter() {
            if let Err(_) = tx.send(msg.to_string()) {
                println!("Failed to send message to peer: {}", peer);
            }
        }
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
