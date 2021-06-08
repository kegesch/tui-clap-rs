use std::io;
use tui::Terminal;
use tui::backend::{CrosstermBackend, Backend};
use tui::widgets::{Widget, Block, Borders};
use tui::layout::{Layout, Constraint, Direction};
use tui_clap::{CommandInput, Events, CommandInputState};
use crossterm::event::{Event, KeyEvent, KeyCode};
use std::sync::mpsc;

fn main() -> Result<(), io::Error> {
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut events = Events::new();
    let mut command_input_state = CommandInputState::default();

    terminal.clear();
    loop {
        draw(&mut terminal, &mut command_input_state)?;
        fetch_event(&mut events, &mut command_input_state);
    }
}

fn fetch_event(events: &mut Events, command_input_state: &mut CommandInputState) -> Result<(), mpsc::RecvError> {
    if let Event::Key(input) = events.next()? {
        match input.code {
            KeyCode::Enter => {}
            KeyCode::Char(char) => {
                command_input_state.add_char(char);
            }
            KeyCode::Backspace => {
                command_input_state.del_char();
            }
            _ => {}
        }
    }
    Ok(())
}

fn draw<B: Backend>(terminal: &mut Terminal<B>, command_input_state: &mut CommandInputState) -> io::Result<()>{
    terminal.draw(|f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Percentage(10),
                    Constraint::Percentage(80),
                    Constraint::Percentage(10)
                ].as_ref()
            )
            .split(f.size());
        let block = Block::default()
            .title("Block")
            .borders(Borders::ALL);
        f.render_widget(block, chunks[0]);
        let block = Block::default()
            .title("Block 2")
            .borders(Borders::ALL);
        f.render_widget(block, chunks[1]);
        let block = Block::default()
            .title("Command")
            .borders(Borders::ALL);
        f.render_widget(block, chunks[2]);
        let command_input = CommandInput::default()
            .prompt("tradrs > ")
            .margin(1);
        f.render_stateful_widget(command_input, chunks[2], command_input_state);
    })?;
    Ok(())
}