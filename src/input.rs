use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyModifiers;
use ratatui::prelude::*;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;
use super::*;

pub enum Command {
    Exit,
}

pub struct Input {
    /// The text currently displayed in the box.
    text: String,

    /// History of past messages, where the most recent
    /// message is at the end. 
    history: Vec<String>,

    /// Whether the currently displayed message is an error.
    is_err: bool,

    /// History cursor index, relative to the most recent message.
    /// If cursor=0, the cursor is not on any message. 
    cursor: usize,

    /// Temporary storage while searching history.
    temp: Option<String>,
}

impl Input {
    pub fn on_key_event(&mut self, ev: KeyEvent) -> Option<Command> {
        if self.is_err {
            self.is_err = false;
            self.text = String::new();
        }

        if ev.code == KeyCode::Char('c') && ev.modifiers.contains(KeyModifiers::CONTROL) {
            self.text = String::new();
            self.cursor = 0;
            self.temp = None;
            return None;
        }

        match ev.code {
            KeyCode::Char(c) => self.text.push(c),                
            KeyCode::Backspace => { self.text.pop(); },
            KeyCode::Up => self.cursor_increment(),
            KeyCode::Down => self.cursor_decrement(),
            KeyCode::Enter => {
                tracing::error!("You pressed Enter!");
                self.temp = None;
                self.cursor = 0;
                let cmd = mem::take(&mut self.text);
                match &*cmd {
                    "exit" | "quit" => return Some(Command::Exit),
                    _ => self.write_err("Unknown Command"),
                }
                self.history.push(cmd);
            },
            _ => {}
        }

        None
    }

    fn write_err(&mut self, e: impl Into<String>) {
        self.is_err = true;
        self.text = e.into();
    }

    /// Increment the cursor to the previous input in the history.
    fn cursor_increment(&mut self) {
        if self.cursor != self.history.len() {
            if self.cursor == 0 {
                self.temp = Some(mem::take(&mut self.text));
            }
            self.cursor += 1;
            self.text = self.history[self.history.len() - self.cursor].clone();
        } 
    }

    /// Decrement the cursor to the next input in the history.
    fn cursor_decrement(&mut self) {
        if self.cursor != 0 {
            if self.cursor == 1 {
                self.cursor = 0;
                self.text = self.temp.take().unwrap_or_default();
            } else {
                self.cursor -= 1;
                self.text = self.history[self.history.len() - self.cursor].clone();
            }
        }
    }
}

impl Default for Input {
    fn default() -> Self {
        Self {
            text: String::new(),
            history: Vec::new(),
            is_err: false,
            cursor: 0,
            temp: None,
        }
    }
}

pub fn render_input_box(frame: &mut Frame, state: &mut State, area: Rect) {
    let style = if state.input.is_err { Style::new().red() } else { Style::new() };
    frame.render_widget(
        Paragraph::new(
            Text::from(&*state.input.text)
                .style(style)
        ).block(Block::bordered().title("Input")),
        area
    );
}