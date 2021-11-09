use clap::{App, ArgMatches, ErrorKind};
use crossterm::event::{poll, read, Event, KeyCode};
use std::borrow::BorrowMut;
use std::cmp::{min};
use std::str::Lines;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{RecvError, TryRecvError};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;
use tui::backend::Backend;
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::Style;
use tui::widgets::{StatefulWidget, Widget};
use tui::Frame;

pub struct Events {
    rx: mpsc::Receiver<Event>,
    ignore_exit_key: Arc<AtomicBool>,
}

#[derive(Default, Clone)]
pub struct CommandInput {
    prompt: String,
}

#[derive(Default)]
pub struct CommandInputState {
    history: Vec<String>,
    index_of_history: usize,
    content: String,
}

#[derive(Default, Clone)]
pub struct CommandOutput {}

#[derive(Default)]
pub struct CommandOutputState {
    history: Vec<String>,
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

    pub fn enter(&mut self) -> String {
        let command = self.content.clone();
        self.history.push(command.clone());
        self.reset();

        command
    }

    pub fn back_in_history(&mut self) {
        self.index_of_history = min(self.index_of_history + 1, self.history.len() - 1);

        self.content = self.history[self.index_of_history].clone();
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
        buf.set_string(
            area.left() + self.prompt.len() as u16,
            area.top(),
            &state.content,
            Style::default(),
        );
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
        let max_chars_per_line = area.width - 1;

        let mut lines_to_render: Vec<&str> = vec![];

        let history_to_show = state.history.iter().rev().take(max_lines as usize).rev();
        let mut y = 0;
        for line in history_to_show {
            if line.len() > max_chars_per_line as usize {
                let mut rest_of_line = line.as_str();
                loop {
                    if rest_of_line.len() > max_chars_per_line as usize {
                        let split_line = rest_of_line.split_at(max_chars_per_line as usize);
                        lines_to_render.push(split_line.0);
                        rest_of_line = split_line.1;
                    } else {
                        lines_to_render.push(rest_of_line);
                        break;
                    }
                }
            } else {
                lines_to_render.push(line);
            }
        }

        for line in lines_to_render.iter().rev().take(max_lines as usize).rev() {
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

impl Default for Events {
    fn default() -> Self {
        Events::from_config(Config::default())
    }
}

impl Events {
    pub fn from_config(config: Config) -> Events {
        let (tx, rx) = mpsc::channel();
        let ignore_exit_key = Arc::new(AtomicBool::new(false));
        {
            let ignore_exit_key = ignore_exit_key.clone();
            thread::spawn(move || loop {
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
            })
        };
        Events {
            rx,
            ignore_exit_key,
        }
    }

    pub fn next(&self) -> Result<Option<Event>, mpsc::RecvError> {
        match self.rx.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(err) => match err {
                TryRecvError::Empty => Ok(None),
                TryRecvError::Disconnected => Err(RecvError {}),
            },
        }
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
    handle_matches: Arc<dyn Fn(ArgMatches) -> Result<Vec<String>, String> + Send + Sync>,
}

impl TuiClap<'_> {
    pub fn from_app<'a>(
        app: App<'a>,
        handle_matches: Arc<
            impl Fn(ArgMatches) -> Result<Vec<String>, String> + 'a + 'static + Send + Sync,
        >,
    ) -> TuiClap {
        TuiClap {
            command_input_state: CommandInputState::default(),
            command_output_state: CommandOutputState::default(),
            command_input_widget: Default::default(),
            command_output_widget: Default::default(),
            clap: app,
            handle_matches,
        }
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
            Ok(matches) => self.handle_matches(matches),
            Err(err) => match err.kind {
                ErrorKind::DisplayHelp => {
                    let mut buf = Vec::new();
                    let mut writer = Box::new(&mut buf);
                    self.clap
                        .write_help(&mut writer)
                        .expect("Could not write help");
                    self.write_to_output(std::str::from_utf8(buf.as_slice()).unwrap().to_string());
                }
                ErrorKind::DisplayVersion => {
                    self.write_to_output(self.clap.render_long_version());
                }
                ErrorKind::Format => {}
                _ => self.write_to_output(format!("error: {}", err)),
            },
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
        frame.render_stateful_widget(
            self.command_input_widget.clone(),
            area,
            self.command_input_state.borrow_mut(),
        );
    }

    pub fn output_widget(&mut self) -> &mut CommandOutput {
        self.command_output_widget.borrow_mut()
    }

    pub fn render_output<B: Backend>(&mut self, frame: &mut Frame<B>, area: Rect) {
        frame.render_stateful_widget(
            self.command_output_widget.clone(),
            area,
            self.command_output_state.borrow_mut(),
        );
    }
}
