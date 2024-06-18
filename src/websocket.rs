use std::sync::Arc;

use base64::{prelude::BASE64_STANDARD, Engine as _};
use sha1::{Digest as _, Sha1};
use tokio::{
    io::{self, AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpStream,
    sync::Mutex,
    task::JoinHandle,
};

use crate::Signal;

impl Opcode {
    fn from_byte(byte: u8) -> Self {
        match byte {
            0x0 => Opcode::Continuation,
            0x1 => Opcode::Text,
            0x2 => Opcode::Binary,
            0x8 => Opcode::Close,
            0x9 => Opcode::Ping,
            0xA => Opcode::Pong,
            _ => Opcode::Other(byte),
        }
    }
}

#[derive(Debug)]
pub struct WebSocketMessage {
    pub opcode: Opcode,
    _payload: Vec<u8>,
}

#[derive(Debug)]
pub enum Opcode {
    Continuation,
    Text,
    Binary,
    Close,
    Ping,
    Pong,
    #[allow(dead_code)]
    Other(u8),
}

/// Chatgpt
pub async fn read_websocket_message(stream: &mut TcpStream) -> Result<WebSocketMessage, io::Error> {
    // Read the first two bytes which contain the frame header
    let mut header = [0; 2];
    stream.read_exact(&mut header).await?;

    // First byte: FIN and opcode
    let fin = header[0] & 0x80 != 0;
    let opcode = Opcode::from_byte(header[0] & 0x0F);

    // Second byte: Mask and payload length
    let mask = header[1] & 0x80 != 0;
    let mut payload_len = (header[1] & 0x7F) as usize;

    // Read extended payload length if necessary
    if payload_len == 126 {
        let mut extended = [0; 2];
        stream.read_exact(&mut extended).await?;
        payload_len = u16::from_be_bytes(extended) as usize;
    } else if payload_len == 127 {
        let mut extended = [0; 8];
        stream.read_exact(&mut extended).await?;
        payload_len = u64::from_be_bytes(extended) as usize;
    }

    // Read the masking key if present
    let mut masking_key = [0; 4];
    if mask {
        stream.read_exact(&mut masking_key).await?;
    }

    // Read the payload data
    let mut payload = vec![0; payload_len];
    stream.read_exact(&mut payload).await?;

    // Unmask the payload if necessary
    if mask {
        for i in 0..payload_len {
            payload[i] ^= masking_key[i % 4];
        }
    }

    // Return the WebSocketMessage
    Ok(WebSocketMessage {
        opcode,
        _payload: payload,
    })
}

/// Chatgpt
pub async fn send_websocket_message(stream: &mut TcpStream, message: &str) -> io::Result<()> {
    let mut frame = Vec::new();
    frame.push(0x81);
    if message.len() < 126 {
        frame.push(message.len() as u8);
    } else if message.len() <= 65535 {
        frame.push(126);
        frame.extend_from_slice(&(message.len() as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(message.len() as u64).to_be_bytes());
    }
    frame.extend_from_slice(message.as_bytes());
    stream.write_all(&frame).await?;
    Ok(())
}

pub fn generate_websocket_accept_key(key: &str) -> String {
    let mut sha1 = Sha1::new();
    let websocket_guid = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    sha1.update(key.as_bytes());
    sha1.update(websocket_guid);
    let result = sha1.finalize();
    BASE64_STANDARD.encode(result)
}

fn spawn_loop(
    stream: Arc<Mutex<TcpStream>>,
    closed: Arc<Mutex<bool>>,
    sender: Arc<Mutex<Option<JoinHandle<()>>>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        while let Ok(msg) = read_websocket_message(&mut *stream.lock().await).await {
            if matches!(msg.opcode, Opcode::Close) {
                break;
            }
        }
        *closed.lock().await = true;
        if let Some(v) = sender.lock().await.as_ref() {
            v.abort();
        }
    })
}

pub async fn handle_websocket(
    mut stream: TcpStream,
    key: String,
    signal: Arc<Signal>,
) -> io::Result<()> {
    let response_key = generate_websocket_accept_key(&key);
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
                    Upgrade: websocket\r\n\
                    Connection: Upgrade\r\n\
                    Sec-WebSocket-Accept: {}\r\n\r\n",
        response_key
    );
    stream.write_all(response.as_bytes()).await?;

    tokio::spawn(async move {
        //TODO: refactor
        let stream = Arc::new(Mutex::new(stream));
        let closed = Arc::new(Mutex::new(false));
        let sender = Arc::new(Mutex::new(None));
        let close = Arc::new(Mutex::new(spawn_loop(
            stream.clone(),
            closed.clone(),
            sender.clone(),
        )));
        let sender2 = sender.clone();
        let v = Some(tokio::spawn(async move {
            loop {
                signal.wait_signal();
                if *closed.lock().await {
                    close.lock().await.abort();
                    break;
                }
                close.lock().await.abort();
                {
                    let mut stream = stream.lock().await;
                    if send_websocket_message(&mut stream, "reload").await.is_err() {
                        close.lock().await.abort();
                        break;
                    }
                }
                *close.lock().await = spawn_loop(stream.clone(), closed.clone(), sender2.clone())
            }
        }));
        *sender.lock().await = v;
    });
    Ok(())
}
