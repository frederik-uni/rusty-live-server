use std::sync::{Condvar, Mutex};

pub struct Signal {
    condvar: Condvar,
    mutex: Mutex<bool>,
}

impl Default for Signal {
    fn default() -> Self {
        Self::new()
    }
}

impl Signal {
    fn new() -> Self {
        Signal {
            condvar: Condvar::new(),
            mutex: Mutex::new(false),
        }
    }

    pub fn send_signal(&self) {
        let mut signal_sent = self.mutex.lock().unwrap();
        *signal_sent = true;
        self.condvar.notify_all();
    }

    pub(crate) fn wait_signal(&self) {
        let mut signal_sent = self.mutex.lock().unwrap();
        while !*signal_sent {
            signal_sent = self.condvar.wait(signal_sent).unwrap();
        }
        *signal_sent = false;
    }
}
