#![allow(async_fn_in_trait)]
mod fs;
mod routing;
mod signal;
mod websocket;

#[cfg(feature = "filesystem-events")]
use std::collections::HashMap;
#[cfg(feature = "filesystem-events")]
use std::io::Read;
#[cfg(feature = "filesystem-events")]
use std::path::Path;
use std::{path::PathBuf, sync::Arc};

use routing::handle_client;
pub use signal::Signal;
use tokio::{io, net::TcpListener};

pub use fs::AsyncFileSystem;
pub use fs::Dir;
pub use fs::FileSystemInterface;
#[cfg(feature = "filesystem-events")]
use notify::event::{CreateKind, ModifyKind};
#[cfg(feature = "filesystem-events")]
use notify::{Event, EventKind, RecursiveMode, Watcher as _};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    #[cfg(feature = "filesystem-events")]
    Notify(notify::Error),
}
impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
#[cfg(feature = "filesystem-events")]
impl From<notify::Error> for Error {
    fn from(value: notify::Error) -> Self {
        Self::Notify(value)
    }
}

#[cfg(feature = "filesystem-events")]
fn b3sum(path: &Path) -> io::Result<blake3::Hash> {
    let mut file = std::fs::File::open(path)?;
    let mut bytes = vec![];
    file.read_to_end(&mut bytes)?;
    Ok(blake3::hash(&bytes))
}

pub async fn serve<T: FileSystemInterface + 'static>(
    path: PathBuf,
    port: u16,
    global: bool,
    signal: Option<Signal>,
    fs: T,
) -> Result<()> {
    let signal = Arc::new(signal.unwrap_or_default());
    #[cfg(feature = "filesystem-events")]
    let s = signal.clone();
    #[cfg(feature = "filesystem-events")]
    let abs_path = std::fs::canonicalize(&path)?;
    #[cfg(feature = "filesystem-events")]
    let mut file_table = HashMap::new();
    #[cfg(feature = "filesystem-events")]
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| match res {
        Ok(mut event) => {
            let kind = event.kind;
            if matches!(
                kind,
                EventKind::Create(CreateKind::File)
                    | EventKind::Modify(ModifyKind::Name(_))
                    | EventKind::Modify(ModifyKind::Data(_))
            ) {
                if let Some(changed_file) = event.paths.pop() {
                    if let Ok(hash) = b3sum(&changed_file) {
                        if let Ok(changed_file) = changed_file.canonicalize() {
                            if let Ok(rel_path) = changed_file.strip_prefix(abs_path.clone()) {
                                let changed = match file_table.entry(changed_file.clone()) {
                                    std::collections::hash_map::Entry::Occupied(v) => {
                                        let mu: &mut blake3::Hash = v.into_mut();
                                        if *mu != hash {
                                            *mu = hash;
                                            true
                                        } else {
                                            false
                                        }
                                    }
                                    std::collections::hash_map::Entry::Vacant(v) => {
                                        v.insert(hash);
                                        true
                                    }
                                };
                                if changed {
                                    s.send_signal(rel_path.to_path_buf());
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(_e) => {
            #[cfg(feature = "log")]
            log::warn!("watch error: {:?}", _e)
        }
    })?;

    #[cfg(feature = "filesystem-events")]
    watcher.watch(path.as_path(), RecursiveMode::Recursive)?;
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
                let fs = fs.clone();
                tokio::spawn(async move {
                    handle_client(stream, path, signal, fs).await;
                });
            }
            Err(_e) => {
                #[cfg(feature = "log")]
                log::warn!("Error accepting connection: {:?}", _e);
            }
        }
    }
}
