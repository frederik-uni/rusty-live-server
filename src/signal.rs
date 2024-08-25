use std::path::PathBuf;

use tokio::sync::broadcast::{channel, Receiver, Sender};

pub struct Signal {
    tx: Sender<PathBuf>,
    rx: Receiver<PathBuf>,
}

impl Default for Signal {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Signal {
    fn clone(&self) -> Self {
        Self {
            rx: self.tx.subscribe(),
            tx: self.tx.clone(),
        }
    }
}

impl Signal {
    fn new() -> Self {
        let (tx, rx) = channel(100);
        Signal { tx, rx }
    }

    pub fn send_signal(&self, file: PathBuf) {
        let _ = self.tx.send(file).unwrap();
    }

    pub(crate) async fn wait_signal(&self) -> PathBuf {
        self.rx.resubscribe().recv().await.unwrap()
    }
}
