use ratatui::crossterm;
use tokio::{runtime::Runtime, sync::mpsc, task};

pub enum Event {
    Crossterm(crossterm::event::Event),
    LogReceived(String),
}

pub struct Events {
    rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,
}

impl Events {
    pub fn new(runtime: &Runtime) -> Self {
        let (tx, rx) = mpsc::channel(64);
        let tx2 = tx.clone();
        runtime.spawn_blocking(move || poll_crossterm(tx2));
        Self { rx, tx }
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

fn poll_crossterm(ev_tx: mpsc::Sender<Event>) {
    loop {
        if let Ok(ev) = crossterm::event::read() {
            if ev_tx.blocking_send(Event::Crossterm(ev)).is_err() {
                break;
            }
        }
    }
}