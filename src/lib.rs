use tui::widgets::{Widget, StatefulWidget};
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::Style;
use std::sync::{mpsc, Arc};
use crossterm::event::{read, Event, KeyCode, poll};
use std::{thread};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use clap::{AppSettings, Clap, App, ArgMatches, Error, ErrorKind};
use std::borrow::BorrowMut;
use tui::Frame;
use tui::backend::{CrosstermBackend, Backend};
use std::io::{Write, BufWriter};
use std::str::Lines;

pub struct Events {
    rx: mpsc::Receiver<Event>,
    input_handle: thread::JoinHandle<()>,
    ignore_exit_key: Arc<AtomicBool>,
}

#[derive(Default, Clone)]
pub struct CommandInput {
    prompt: String,
}

#[derive(Default)]
pub struct CommandInputState {
    content: String,
}

#[derive(Default, Clone)]
pub struct CommandOutput {
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
    pub fn prompt(&mut self, prompt: &str) {
        self.prompt = prompt.to_string();
    }
}

impl StatefulWidget for CommandInput {
    type State = CommandInputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_string(area.left(), area.top(), &self.prompt, Style::default());
        buf.set_string(area.left() + self.prompt.len() as u16, area.top(), &state.content, Style::default());
    }
}

impl Widget for CommandInput {
    fn render(self, area: Rect, buf: &mut Buffer) {
        StatefulWidget::render(self, area, buf, &mut CommandInputState::default())
    }
}

impl StatefulWidget for CommandOutput {
    type State = CommandOutputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let max_lines = area.height - 1;

        let history_to_show = state.history.iter().rev().take(max_lines as usize).rev();
        let mut y = 0;
        for line in history_to_show {
            buf.set_string(area.left(), area.top() + y, line, Style::default());
            y += 1;
        }
    }
}

impl Widget for CommandOutput {
    fn render(self, area: Rect, buf: &mut Buffer) {
        StatefulWidget::render(self, area, buf, &mut CommandOutputState::default())
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



pub struct TuiClap<'a> {
    command_input_state: CommandInputState,
    command_output_state: CommandOutputState,
    command_input_widget: CommandInput,
    command_output_widget: CommandOutput,
    clap: App<'a>,
    events: Events,
    handle_matches: Box<dyn Fn(ArgMatches) -> Result<Vec<String>, String>>,
}

impl TuiClap<'_> {
    pub fn from_app<'a>(app: App<'a>, handle_matches: impl Fn(ArgMatches) -> Result<Vec<String>, String>  + 'a + 'static) -> TuiClap {
        TuiClap {
            command_input_state: CommandInputState::default(),
            command_output_state: CommandOutputState::default(),
            command_input_widget: Default::default(),
            command_output_widget: Default::default(),
            clap: app,
            events: Events::new(),
            handle_matches: Box::new(handle_matches)
        }
    }

    pub fn fetch_event(&mut self) -> Result<(), mpsc::RecvError> {
        if let Event::Key(input) = self.events.next()? {
            match input.code {
                KeyCode::Enter => {
                    self.parse();
                    self.command_input_state.content.clear();
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

    pub fn write_to_output(&mut self, string: String) {
        let lines: Lines = string.lines();
        for str in lines {
            self.command_output_state.history.push(str.to_string());
        }
    }

    pub fn state(&mut self) -> &mut CommandInputState {
        self.command_input_state.borrow_mut()
    }

    pub fn parse(&mut self) {
        let content = self.command_input_state.content.clone();
        let commands_vec = content.split(' ').collect::<Vec<&str>>();
        let matches_result = self.clap.try_get_matches_from_mut(commands_vec.clone());

        match matches_result {
            Ok(matches) => {
                self.handle_matches(matches)
            }
            Err(err) => {
                match err.kind {
                    ErrorKind::DisplayHelp => {
                        let mut buf = Vec::new();
                        let mut writer = Box::new(&mut buf);
                        self.clap.write_help(&mut writer);
                        self.write_to_output(std::str::from_utf8(buf.as_slice()).unwrap().to_string());
                    }
                    ErrorKind::DisplayVersion => {
                        self.write_to_output(self.clap.render_long_version());
                    }
                    ErrorKind::Format => {}
                    _ => {
                        self.write_to_output(format!("error: {}", err))
                    }
                }
            }
        }


    }

    fn handle_matches(&mut self, matches: ArgMatches) {
        let handle = &self.handle_matches;
        let output_res: Result<Vec<String>, String> = handle(matches);
        if let Ok(output) = output_res {
            for out in output {
                self.write_to_output(out);
            }
        } else {
            self.write_to_output(output_res.unwrap_err())
        }

    }

    pub fn input_widget(&mut self) -> &mut CommandInput {
        self.command_input_widget.borrow_mut()
    }

    pub fn render_input<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        frame.render_stateful_widget(self.command_input_widget.clone(), area, self.command_input_state.borrow_mut());
    }

    pub fn output_widget(&mut self) -> &mut CommandOutput {
        self.command_output_widget.borrow_mut()
    }

    pub fn render_output<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        frame.render_stateful_widget(self.command_output_widget.clone(), area, self.command_output_state.borrow_mut());
    }
}