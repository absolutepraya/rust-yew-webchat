use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{self, Duration};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};

type UserId = String;
type Tx = mpsc::UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<UserId, (Tx, bool)>>>;

// Message types for the protocol
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebSocketMessage {
    message_type: MessageType,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data_array: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum MessageType {
    Register,
    Users,
    Message,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    from: String,
    message: String,
    time: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<ReplyData>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ReplyData {
    id: usize,
    from: String,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct MessageData {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<String>,
}

async fn handle_connection(
    peer_map: PeerMap,
    raw_stream: TcpStream,
    addr: SocketAddr,
) {
    info!("Incoming connection from: {}", addr);

    let ws_stream = match accept_async(raw_stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("Error during WebSocket handshake: {}", e);
            return;
        }
    };

    info!("WebSocket connection established with: {}", addr);

    let (tx, rx) = mpsc::unbounded_channel();
    let (mut outgoing, mut incoming) = ws_stream.split();

    // Forward messages received on the mpsc channel to the WebSocket
    let forward_task = tokio::spawn(async move {
        let mut rx = rx;
        while let Some(message) = rx.recv().await {
            if let Err(e) = outgoing.send(message).await {
                error!("Error sending message: {}", e);
                break;
            }
        }
    });

    // Process incoming WebSocket messages
    let mut user_id = String::new();
    
    while let Some(result) = incoming.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                error!("Error receiving message: {}", e);
                break;
            }
        };

        if let Message::Text(text) = msg {
            if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                match ws_msg.message_type {
                    MessageType::Register => {
                        if let Some(username) = ws_msg.data {
                            user_id = username.clone();
                            
                            // Add user to the peer map
                            peer_map.lock().unwrap().insert(user_id.clone(), (tx.clone(), true));
                            
                            // Broadcast updated user list
                            broadcast_user_list(&peer_map);
                        }
                    }
                    MessageType::Message => {
                        if let Some(data) = ws_msg.data {
                            if let Ok(msg_data) = serde_json::from_str::<MessageData>(&data) {
                                // Process the message
                                let mut reply_data = None;
                                
                                // Parse reply data if present
                                if let Some(reply_json) = msg_data.reply_to {
                                    if let Ok(reply) = serde_json::from_str::<ReplyData>(&reply_json) {
                                        reply_data = Some(reply);
                                    }
                                }
                                
                                // Create chat message
                                let chat_msg = ChatMessage {
                                    from: user_id.clone(),
                                    message: msg_data.text,
                                    time: SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis() as u64,
                                    reply_to: reply_data,
                                };
                                
                                // Broadcast the message to all clients
                                let message_json = serde_json::to_string(&WebSocketMessage {
                                    message_type: MessageType::Message,
                                    data: Some(serde_json::to_string(&chat_msg).unwrap()),
                                    data_array: None,
                                }).unwrap();
                                
                                broadcast_message(&peer_map, &message_json);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // User disconnected, remove from peer map
    peer_map.lock().unwrap().remove(&user_id);
    broadcast_user_list(&peer_map);
    
    // Cancel the forward task when the connection is closed
    forward_task.abort();
    info!("Connection closed for: {}", addr);
}

fn broadcast_message(peer_map: &PeerMap, message: &str) {
    let peers = peer_map.lock().unwrap();
    
    for (_, (tx, _)) in peers.iter() {
        if let Err(e) = tx.send(Message::Text(message.to_string())) {
            error!("Error broadcasting message: {}", e);
        }
    }
}

fn broadcast_user_list(peer_map: &PeerMap) {
    let peers = peer_map.lock().unwrap();
    let user_list: Vec<String> = peers.keys().cloned().collect();
    
    let users_message = WebSocketMessage {
        message_type: MessageType::Users,
        data: None,
        data_array: Some(user_list),
    };
    
    let json = serde_json::to_string(&users_message).unwrap();
    
    for (_, (tx, _)) in peers.iter() {
        if let Err(e) = tx.send(Message::Text(json.clone())) {
            error!("Error broadcasting user list: {}", e);
        }
    }
}

async fn check_connections(peer_map: PeerMap) {
    // Timeout for checking connections
    let interval = Duration::from_secs(5);
    let mut interval_stream = time::interval(interval);
    
    loop {
        interval_stream.tick().await;
        let mut peers = peer_map.lock().unwrap();
        let mut changed = false;
        
        // Check which connections are still alive
        let peers_to_remove: Vec<String> = peers
            .iter()
            .filter(|(_, (_, is_alive))| !*is_alive)
            .map(|(id, _)| id.clone())
            .collect();
        
        // Remove disconnected peers
        for id in peers_to_remove {
            peers.remove(&id);
            changed = true;
        }
        
        // Mark all connections as not alive for next check
        for (_, (_, is_alive)) in peers.iter_mut() {
            *is_alive = false;
        }
        
        // Drop the lock before broadcasting
        drop(peers);
        
        // If users changed, broadcast new user list
        if changed {
            broadcast_user_list(&peer_map);
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.expect("Failed to bind to address");
    info!("WebSocket server listening on: {}", addr);
    
    let peer_map = PeerMap::new(Mutex::new(HashMap::new()));
    
    // Spawn the connection checker
    let peer_map_clone = peer_map.clone();
    tokio::spawn(async move {
        check_connections(peer_map_clone).await;
    });
    
    // Accept and handle new connections
    while let Ok((stream, addr)) = listener.accept().await {
        let peer_map_clone = peer_map.clone();
        tokio::spawn(async move {
            handle_connection(peer_map_clone, stream, addr).await;
        });
    }
} 