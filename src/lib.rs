use tui::widgets::{Widget, StatefulWidget};
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::Style;
use std::sync::{mpsc, Arc};
use crossterm::event::{read, Event, KeyCode, poll};
use std::{thread};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use clap::{AppSettings, Clap, App};
use std::borrow::BorrowMut;
use tui::Frame;
use tui::backend::CrosstermBackend;
use std::io::Write;

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

#[derive(Default)]
pub struct CommandOutputState {
    history: Vec<String>
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



pub struct TuiClap {
    command_input_state: CommandInputState,
    command_output_state: CommandOutputState,
    command_input_widget: CommandInput,
    clap: App<'static>,
    events: Events,
}

impl TuiClap {
    pub fn from_app(app: App<'static>) -> TuiClap {
        TuiClap {
            command_input_state: CommandInputState::default(),
            command_output_state: CommandOutputState::default(),
            command_input_widget: Default::default(),
            clap: app,
            events: Events::new(),
        }
    }

    pub fn fetch_event(&mut self) -> Result<(), mpsc::RecvError> {
        if let Event::Key(input) = self.events.next()? {
            match input.code {
                KeyCode::Enter => {
                    self.parse()
                }
                KeyCode::Char(char) => {
                    self.command_input_state.add_char(char);
                }
                KeyCode::Backspace => {
                    self.command_input_state.del_char();
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn state(&mut self) -> &mut CommandInputState {
        self.command_input_state.borrow_mut()
    }

    pub fn parse(&mut self) {
        let commands_vec = self.command_input_state.content.split(' ');
        let matches = self.clap.clone().get_matches_from(commands_vec);

        let config = matches.value_of("config").unwrap_or("default.conf");
        println!("Value for config: {}", config);

        // Calling .unwrap() is safe here because "INPUT" is required (if "INPUT" wasn't
        // required we could have used an 'if let' to conditionally get the value)
        println!("Using input file: {}", matches.value_of("INPUT").unwrap());

        // Vary the output based on how many times the user used the "verbose" flag
        // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
        match matches.occurrences_of("v") {
            0 => println!("No verbose info"),
            1 => println!("Some verbose info"),
            2 => println!("Tons of verbose info"),
            3 | _ => println!("Don't be crazy"),
        }

        // You can handle information about subcommands by requesting their matches by name
        // (as below), requesting just the name used, or both at the same time
        if let Some(matches) = matches.subcommand_matches("test") {
            if matches.is_present("debug") {
                println!("Printing debug info...");
            } else {
                println!("Printing normally...");
            }
        }

        self.command_input_state.content = String::new();
    }

    pub fn input_widget(&mut self) -> &mut CommandInput {
        self.input_widget().borrow_mut()
    }

    pub fn render_input<W: Write>(&mut self, frame: &mut Frame<CrosstermBackend<W>>, area: Rect) {
        frame.render_stateful_widget(self.command_input_widget.clone(), area, self.command_input_state.borrow_mut());
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
