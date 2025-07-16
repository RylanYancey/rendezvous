use ratatui::{prelude::*, widgets::{Block, Clear, List, ListDirection, ListItem, Paragraph, Wrap}};
use std::{collections::VecDeque, fs, io::{self, BufRead}, path::{Path, PathBuf}, sync::{Arc, LazyLock}};
use parking_lot::Mutex;
use tracing::level_filters::LevelFilter;
use crate::event::Event;
use super::*;
use tokio::sync::mpsc;
use tracing_subscriber::{fmt::{format::Writer, FmtContext, FormatEvent, MakeWriter}, layer::{Filter, SubscriberExt}, util::SubscriberInitExt, Layer, Registry};

pub struct Logs {
    pub init_error: Option<String>,
    pub items: VecDeque<String>,
}

impl Logs {
    fn err(e: impl Into<String>) -> Self {
        Self { 
            init_error: Some(e.into()), 
            items: VecDeque::new(),
        }
    }

    pub fn get_text(&mut self, rect: Rect) -> impl Iterator<Item=Line<'_>> {
        self.items
            .iter()
            .rev()
            .take(rect.height as usize - 2)
            .rev()
            .map(|s| Line::from(s.as_str()))
    }

    pub fn update(&mut self, log: String) {
        self.items.push_back(log);
        if self.items.len() > 100 {
            self.items.pop_front();
        }
    }

    pub fn init(ev_tx: mpsc::Sender<Event>) -> Self {
        // get data directory
        let dir = match dirs::data_dir() {
            Some(dir) => dir.join(crate::NAME),
            None => return Self::err("[E391] Failed to find a suitable directory for log data.")
        };

        // create the directory if it doesn't exist.
        if let Err(e) = fs::create_dir_all(&dir) {
            return Self::err(format!("[E392] Failed to initialize data directory with error: '{e}'"))
        }

        // path to log file
        let logs_path = dir.join("logs").with_extension("txt");

        // load the 100 most recent logs.
        let items = match load_recent_logs(&logs_path) {
            Ok(mut items) => {
                if items.len() > 0 {
                    items.push(format!("### SESSION START ###"));
                }
                items
            },
            Err(e) => return Self::err(format!("[E394] Failed to load recent logs with error: '{e}'")),
        };

        // open for writing new logs
        let logs_file = match fs::OpenOptions::new().append(true).open(logs_path) {
            Ok(file) => file,
            Err(e) => return Self::err(format!("[E393] Failed to open log file with error: '{e}'")),
        };

        let sub = tracing_subscriber::fmt::layer()
            .with_file(false)
            .with_line_number(false)
            .with_ansi(true)
            .with_target(false)
            .without_time()
            .with_level(true)
            .log_internal_errors(false)
            .compact()
            .with_writer(
                LogWriter {
                    ev_tx,
                    file: Arc::new(Mutex::new(logs_file)),
                    buffer: Arc::new(Mutex::new(String::new()))
                }
            );

        if let Err(e) = tracing::subscriber::set_global_default(
            Registry::default()
                .with(LevelFilter::INFO)
                .with(sub)
        ) {
            return Self::err(format!("[E394] Failed to configure logger with error: '{e}'."));
        }

        Self {
            init_error: None,
            items: VecDeque::from_iter(items.into_iter()),
        }
    }
}

pub fn render_logs_box(frame: &mut Frame, state: &mut State, area: Rect) {
    let block = Block::bordered().title("Logs");

    if let Some(err) = &state.logs.init_error {
        frame.render_widget(
            Paragraph::new(Text::from(err.as_str()).red()).block(block),
            area
        );
    } else {
        let text = state.logs.get_text(area);
        let widget = Paragraph::new(Text::from_iter(text))
            .block(block)
            .left_aligned()
            .wrap(Wrap { trim: false });

        frame.render_widget(Clear, area);
        frame.render_widget(widget, area);
    }
}

#[derive(Clone)]
struct LogWriter {
    ev_tx: mpsc::Sender<Event>,
    file: Arc<Mutex<fs::File>>,
    buffer: Arc<Mutex<String>>,
}

impl<'a> MakeWriter<'a> for LogWriter {
    type Writer = LogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // write bytes to log file
        self.file.lock().write_all(&buf)?;

        let s = match std::str::from_utf8(buf) {
            Ok(s) => s,
            Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8")),
        };

        let mut buffer = self.buffer.lock();
        buffer.push_str(s);

        while let Some(pos) = buffer.find('\n') {
            let line = buffer.drain(..=pos).collect::<String>();
            let _ = self.ev_tx.blocking_send(Event::LogReceived(line));
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Loads the most recent 100 logs and trims excess logs. 
fn load_recent_logs(path: &Path) -> io::Result<Vec<String>> {
    use io::Write;
    use io::BufReader;

    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e)
    };

    // Read all lines
    let reader = BufReader::new(&mut file);
    let mut lines: Vec<String> = reader.lines().filter_map(Result::ok).collect();

    // Trim to max 300 lines
    if lines.len() > 300 {
        lines = lines.split_off(lines.len() - 300);

        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path)?;

        for line in &lines {
            writeln!(file, "{}", line)?;
        }
    }

    // Return last 100 lines
    let recent = lines
        .iter()
        .rev()
        .take(100)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    Ok(recent)
}