use std::mem;
use clap::Parser;
use ratatui::{crossterm, prelude::*};
use tokio::runtime::Runtime;

use crate::{event::{Event, Events}, input::Command, logs::Logs};

pub mod input;
pub mod event;
pub mod logs;

pub const NAME: &'static str = "p2p-rendezvous-server";

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    port: u64,
}

pub struct State {
    input: input::Input,
    output: Option<String>,
    logs: Logs,
}

fn main() {
    let cli = Cli::parse();
    let runtime = Runtime::new().expect("Failed to start async runtime.");
    let mut terminal = ratatui::init();
    let mut events = Events::new(&runtime);
    let mut state = State {
        input: input::Input::default(),
        logs: Logs::init(events.tx()),
        output: None,
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

    let [output_area, logs_area] = horizon.areas(content_area);
    logs::render_logs_box(frame, state, logs_area);
    render_output_box(frame, state, output_area);
}

fn render_output_box(frame: &mut Frame, state: &mut State, area: Rect) {

}