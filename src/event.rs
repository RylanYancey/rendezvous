use ratatui::crossterm;
use tokio::{runtime::Runtime, sync::mpsc, task};

use crate::startup::{StartupError, StartupEvent};

pub enum Event {
    Crossterm(crossterm::event::Event),
    LogReceived(String),
    Startup(StartupEvent),
    StartupError(StartupError),
}

pub struct Events {
    rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,
}

impl Events {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(64);
        Self { rx, tx }
    }

    pub fn start_polling_crossterm(&self, runtime: &Runtime) {
        let ev_tx = self.tx();
        runtime.spawn_blocking(move || {
            loop {
                if let Ok(ev) = crossterm::event::read() {
                    if ev_tx.blocking_send(Event::Crossterm(ev)).is_err() {
                        break;
                    }
                }
            }
        });
    }

    /// Returns None when there are no more events to read.
    pub fn read(&mut self) -> Option<Event> {
        self.rx.blocking_recv()
    }

    /// Get a transmitter for events.
    pub fn tx(&self) -> mpsc::Sender<Event> {
        self.tx.clone()
    }
}
