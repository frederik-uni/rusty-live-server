mod routing;
mod signal;
mod websocket;

use std::{path::PathBuf, sync::Arc};

use routing::handle_client;
pub use signal::Signal;
use tokio::{io, net::TcpListener};

#[cfg(feature = "filesystem-events")]
use notify::event::{CreateKind, ModifyKind};
#[cfg(feature = "filesystem-events")]
use notify::{Event, EventKind, RecursiveMode, Watcher as _};

pub async fn serve(
    path: PathBuf,
    port: u16,
    global: bool,
    signal: Option<Signal>,
) -> io::Result<()> {
    let signal = Arc::new(signal.unwrap_or_default());
    #[cfg(feature = "filesystem-events")]
    let s = signal.clone();
    #[cfg(feature = "filesystem-events")]
    let mut watcher =
        notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
            Ok(event) => {
                let kind = event.kind;
                if matches!(
                    kind,
                    EventKind::Create(CreateKind::File)
                        | EventKind::Modify(ModifyKind::Name(_))
                        | EventKind::Modify(ModifyKind::Data(_))
                ) {
                    s.send_signal();
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        })
        .unwrap();

    #[cfg(feature = "filesystem-events")]
    watcher
        .watch(path.as_path(), RecursiveMode::Recursive)
        .unwrap();
    let addr = match global {
        true => format!("0.0.0.0:{port}"),
        false => format!("127.0.0.1:{port}"),
    };
    let listener = TcpListener::bind(addr).await?;
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let path = path.clone();
                let signal = signal.clone();
                tokio::spawn(async move {
                    handle_client(stream, path, signal).await;
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {:?}", e);
            }
        }
    }
}
