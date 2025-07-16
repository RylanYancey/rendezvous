use std::{mem, net::Ipv4Addr};
use clap::Parser;
use ratatui::{crossterm, prelude::*, widgets::{Block, Clear, Paragraph}};
use tokio::runtime::Runtime;
use crate::{event::{Event, Events}, input::Command, logs::Logs, startup::{StartupError, StartupEvent}};

pub mod matchbox;
pub mod startup;
pub mod input;
pub mod event;
pub mod logs;

pub const NAME: &'static str = "p2p-rendezvous-server";

#[derive(Parser)]
struct Cli {
    #[arg(long, short)]
    port: Option<u16>,
}

pub struct State {
    input: input::Input,
    output: Option<String>,
    startup: Vec<StartupEvent>,
    status: Status,
    logs: Logs,
}

pub enum Status {
    Starting,
    StartupFailed(StartupError),
    Running {
        local_ip: Ipv4Addr,
        public_ip: Ipv4Addr,
    }
}

fn main() {
    let cli = Cli::parse();
    let mut events = Events::new();
    let logs = Logs::init(events.tx());
    let runtime = Runtime::new().expect("Failed to start async runtime.");
    let mut terminal = ratatui::init();
    // initiate startup sequence
    let startup_tx = events.tx();
    runtime.spawn(async move { startup::startup(cli.port, startup_tx).await });
    // start polling task
    events.start_polling_crossterm(&runtime);
    let mut state = State {
        input: input::Input::default(),
        logs,
        output: None,
        status: Status::Starting,
        startup: Vec::new(),
    };
    'outer: loop {
        terminal.draw(|frame| draw(frame, &mut state)).unwrap();
        match events.read() {
            None => break 'outer,
            Some(ev) => {
                match ev {
                    Event::Crossterm(crossterm::event::Event::Key(ev)) => {
                        if let Some(cmd) = state.input.on_key_event(ev) {
                            match cmd {
                                Command::Exit => break 'outer,
                            }
                        }
                    },
                    Event::Crossterm(crossterm::event::Event::Resize(_, _)) => {}
                    Event::LogReceived(log) => state.logs.update(log),
                    Event::Startup(ev) => state.startup.push(ev),
                    Event::StartupError(e) => state.status = Status::StartupFailed(e),
                    _ => {}
                }
            }
        }
    }
    runtime.shutdown_background();
    ratatui::restore();
}

fn draw(frame: &mut Frame, state: &mut State) {
    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(3),
    ]);

    let [content_area, input_area] = vertical.areas(frame.area());
    input::render_input_box(frame, state, input_area);

    let horizon = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ]);

    let [content_area, logs_area] = horizon.areas(content_area);
    logs::render_logs_box(frame, state, logs_area);

    let vertical = Layout::vertical([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ]);
    let [output_area, status_area] = vertical.areas(content_area);
    render_output_box(frame, state, output_area);
    render_status_box(frame, state, status_area);
}

fn render_output_box(frame: &mut Frame, state: &mut State, area: Rect) {
    let block = Block::bordered().title("Output");
    if let Some(output) = state.output.take() {
        frame.render_widget(Paragraph::new(output).block(block), area);
    } else {
        frame.render_widget(block, area)
    }
}

fn render_status_box(frame: &mut Frame, state: &mut State, area: Rect) {
    let mut text = Vec::new();
    match &state.status {
        Status::Starting => {
            text.push("Status: Starting".into());

            for ev in &state.startup {
                if let StartupEvent::ProgressHint(hint) = ev {
                    text.push(hint.clone());
                }
            }
        },
        Status::StartupFailed(e) => {
            text.push("Status: ERROR".into());
            text.push(format!("Startup failed with error: '{e:?}'"));
        },
        Status::Running { local_ip, public_ip } => {
            text.push("Status: Running".into());
            text.push(format!("Local IP: {}", local_ip));
            text.push(format!("Public IP: {}", public_ip));
        }
    }

    let block = Block::bordered().title("Status");
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text.join("\n"))
            .block(block),
        area
    );
}
