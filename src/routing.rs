use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::{
    fs::{read_dir, File},
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpStream,
};

use crate::{websocket::handle_websocket, Signal};

pub async fn handle_client(mut stream: TcpStream, base_dir: PathBuf, signal: Arc<Signal>) {
    let mut buffer = [0; 512];
    if stream.read(&mut buffer).await.is_ok() {
        let request = String::from_utf8_lossy(&buffer[..]);
        let mut parts = request.split_whitespace();
        let protocol = parts.next();
        if let (Some("GET"), Some(path), Some(_)) = (protocol, parts.next(), parts.next()) {
            let mut file_path = base_dir.to_path_buf();
            let mut websocket = None;
            let path = path.split_once('?').map(|v| v.0).unwrap_or(path);
            if path != "/" {
                file_path.push(&path[1..]);
            }
            if path == "/ws" {
                while let Some(header) = parts.next() {
                    if header == "Sec-WebSocket-Key:" {
                        if let Some(next_header) = parts.next() {
                            websocket = Some(next_header.to_string());
                        }
                        break;
                    }
                }
            }
            if let Some(key) = websocket {
                let _ = handle_websocket(stream, key, signal).await;
            } else if file_path.is_dir() {
                if serve_directory(&file_path, &mut stream).await.is_err() {
                    serve_500(&mut stream).await;
                }
            } else if file_path.is_file() {
                if serve_file(&file_path, &mut stream).await.is_err() {
                    serve_500(&mut stream).await;
                }
            } else {
                serve_404(&mut stream).await;
            }
        }
    }
}

async fn serve_directory(dir: &Path, stream: &mut TcpStream) -> io::Result<()> {
    let mut response = String::new();
    response.push_str("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n");
    response.push_str("<html><body><ul>");

    let mut entries = read_dir(dir).await?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let file_name = entry.file_name().into_string().unwrap_or_default();
        response.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>",
            file_name, file_name
        ));
    }

    response.push_str("</ul></body></html>");
    let _ = stream.write(response.as_bytes()).await?;
    Ok(())
}

async fn serve_file(file_path: &Path, stream: &mut TcpStream) -> io::Result<()> {
    let mut file = File::open(file_path).await?;
    let is_html = file_path
        .as_os_str()
        .to_str()
        .unwrap_or_default()
        .ends_with(".html");
    let mut contents = Vec::new();
    if is_html {
        contents.append(
            &mut format!("<script>{}</script>", include_str!("updater.js"))
                .as_bytes()
                .to_vec(),
        )
    }
    let _ = file.read_to_end(&mut contents).await?;

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
        contents.len()
    );
    let _ = stream.write(response.as_bytes()).await;
    let _ = stream.write(&contents).await;
    Ok(())
}

async fn serve_404(stream: &mut TcpStream) {
    let response = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
    let _ = stream.write(response.as_bytes()).await;
}

async fn serve_500(stream: &mut TcpStream) {
    let response = "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";
    let _ = stream.write_all(response.as_bytes()).await;
}
