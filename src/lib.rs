use tui::widgets::{Widget, StatefulWidget};
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::Style;
use std::sync::{mpsc, Arc};
use crossterm::event::{read, Event, KeyCode, poll};
use std::{thread};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub struct Events {
    rx: mpsc::Receiver<Event>,
    input_handle: thread::JoinHandle<()>,
    ignore_exit_key: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct CommandInput {
    prompt: String,
    margin: u16,
}

#[derive(Default)]
pub struct CommandInputState {
    content: String,
}

impl CommandInputState {
    pub fn add_char(&mut self, c: char) {
        self.content.push(c);
    }

    pub fn del_char(&mut self) {
        self.content.pop();
    }

    pub fn reset(&mut self) {
        self.content.drain(..);
    }
}

impl CommandInput {
    pub fn prompt(mut self, prompt: &str) -> Self {
        self.prompt = prompt.to_string();
        self
    }

    pub fn margin(mut self, margin: u16) -> Self {
        self.margin = margin;
        self
    }
}

impl StatefulWidget for CommandInput {
    type State = CommandInputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_string(area.left() + self.margin, area.top() + self.margin, &self.prompt, Style::default());
        buf.set_string(area.left() + self.margin + self.prompt.len() as u16, area.top() + self.margin, &state.content, Style::default());
    }
}

impl Widget for CommandInput {
    fn render(self, area: Rect, buf: &mut Buffer) {
        StatefulWidget::render(self, area, buf, &mut CommandInputState::default())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub exit_key: KeyCode,
    pub tick_rate: Duration,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            exit_key: KeyCode::Char('q'),
            tick_rate: Duration::from_millis(250),
        }
    }
}

impl Events {
    pub fn new() -> Events {
        Events::with_config(Config::default())
    }

    pub fn with_config(config: Config) -> Events {
        let (tx, rx) = mpsc::channel();
        let ignore_exit_key = Arc::new(AtomicBool::new(true));
        let input_handle = {
            let tx = tx.clone();
            let ignore_exit_key = ignore_exit_key.clone();
            thread::spawn(move || {
                loop {
                    if let Ok(b) = poll(config.tick_rate) {
                        if !b {
                            continue;
                        }
                        let read = read();
                        if let Ok(event) = read {
                            if let Err(err) = tx.send(event) {
                                eprintln!("{}", err);
                                return;
                            }
                            if !ignore_exit_key.load(Ordering::Relaxed) {
                                if let Event::Key(key) = event {
                                    if key.code == config.exit_key {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            })
        };
        Events {
            rx,
            ignore_exit_key,
            input_handle,
        }
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }

    pub fn disable_exit_key(&mut self) {
        self.ignore_exit_key.store(true, Ordering::Relaxed);
    }

    pub fn enable_exit_key(&mut self) {
        self.ignore_exit_key.store(false, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
