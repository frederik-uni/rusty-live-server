use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    net::TcpStream,
};

use crate::{fs::File, websocket::handle_websocket, Dir, FileSystemInterface, Signal};

pub async fn handle_client(
    mut stream: TcpStream,
    base_dir: PathBuf,
    signal: Arc<Signal>,
    fs: impl FileSystemInterface,
) {
    let mut reader = BufReader::new(&mut stream);
    let mut request = String::new();
    let mut line = String::new();

    loop {
        let bytes_read = reader.read_line(&mut line).await;
        let bytes_read = match bytes_read {
            Ok(v) => v,
            Err(_) => {
                break;
            }
        };
        if bytes_read == 0 {
            break;
        }

        request.push_str(&line);
        line.clear();
        if request.contains("\r\n\r\n") {
            break;
        }
    }
    let mut parts = request.split_whitespace();
    let protocol = parts.next();
    let temp = (protocol, parts.next(), parts.next());
    if let (Some(method), Some(path), Some(_)) = temp {
        if matches!(method, "GET" | "HEAD") {
            let head = method == "HEAD";
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
            } else if path == "/favicon.ico" {
                serve_favicon(&file_path, &mut stream, fs, head).await;
            } else if file_path.is_dir() {
                if serve_directory(&file_path, &mut stream, fs, head)
                    .await
                    .is_err()
                {
                    serve_500(&mut stream).await;
                }
            } else if file_path.is_file() {
                if serve_file(&file_path, &mut stream, fs, head).await.is_err() {
                    serve_500(&mut stream).await;
                }
            } else {
                serve_404(&mut stream).await;
            }
        }
    } else if let (Some("POST"), Some("/ping"), Some(_)) = temp {
        let contents = "pong";
        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text\r\n\r\n{}", contents);
        let _ = stream.write(response.as_bytes()).await;
    }
}

async fn serve_directory(
    dir: &Path,
    stream: &mut TcpStream,
    fs: impl FileSystemInterface,
    head: bool,
) -> crate::Result<()> {
    let mut response = String::new();
    response.push_str("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n");
    if head {
        let _ = stream.write(response.as_bytes()).await?;
        return Ok(());
    }
    response.push_str("<html><body><ul>");
    let mut entries = fs.get_dir(dir).await?;
    let mut found_index = None;
    while let Ok(Some(entry)) = entries.get_next().await {
        let file_name = entry
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or_default();
        if file_name == "index.html" {
            found_index = Some(dir.join("index.html"));
            break;
        }
        response.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>",
            file_name, file_name
        ));
    }

    drop(entries);
    if let Some(found) = found_index {
        return Ok(serve_file(&found, stream, fs, head).await?);
    }

    response.push_str("</ul></body></html>");
    let _ = stream.write(response.as_bytes()).await?;
    Ok(())
}

async fn serve_file(
    file_path: &Path,
    stream: &mut TcpStream,
    fs: impl FileSystemInterface,
    head: bool,
) -> crate::Result<()> {
    let is_html = file_path
        .as_os_str()
        .to_str()
        .unwrap_or_default()
        .ends_with(".html");
    let mime = mime_guess::from_path(file_path).first_or_octet_stream();

    let mut contents = fs.get_file(file_path).await?.read_to_end().await;
    if is_html {
        contents.append(
            &mut format!("<script defer>{}</script>", include_str!("updater.js"))
                .as_bytes()
                .to_vec(),
        )
    }

    let response = format!(
        "HTTP/1.1 200 OK\r\nnContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        mime,
        contents.len()
    );
    let _ = stream.write(response.as_bytes()).await;
    if head {
        return Ok(());
    }
    let _ = stream.write(&contents).await;
    Ok(())
}

async fn serve_404(stream: &mut TcpStream) {
    let response = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
    let _ = stream.write(response.as_bytes()).await;
}

async fn serve_500(stream: &mut TcpStream) {
    let response = "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";
    let _ = stream.write(response.as_bytes()).await;
}

async fn serve_favicon(
    path: &Path,
    stream: &mut TcpStream,
    fs: impl FileSystemInterface,
    head: bool,
) {
    let bytes = match fs.get_file(path).await {
        Ok(mut v) => v.read_to_end().await,
        Err(_) => include_bytes!("../favicon.ico").to_vec(),
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: image/x-icon\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        bytes.len()
    );
    let _ = stream.write(response.as_bytes()).await;
    if head {
        return;
    }
    let _ = stream.write(&bytes).await;
}
