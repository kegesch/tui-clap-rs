use std::io;
use tui::Terminal;
use tui::backend::{CrosstermBackend, Backend};
use tui::widgets::{Block, Borders};
use tui::layout::{Layout, Constraint, Direction, Rect};
use tui_clap::TuiClap;
use clap::{AppSettings, Clap, App, Arg, ArgMatches, load_yaml};


#[derive(Clap)]
#[clap(version = "1.0", author = "Max M. <max@m.com>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    /// Sets a custom config file. Could have been an Option<T> with no default too
    #[clap(short, long, default_value = "default.conf")]
    config: String,
    /// Some input. Because this isn't an Option<T> it's required to be used
    input: String,
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
}

fn main() -> Result<(), io::Error> {
    let yaml = load_yaml!("cli.yaml");
    let app = App::from(yaml);

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut tui = TuiClap::from_app(app, handle_matches);
    tui.input_widget().prompt("tradrs > ");

    terminal.clear();
    loop {
        draw(&mut terminal, &mut tui)?;
        tui.fetch_event();
    }
}

fn draw<B: Backend>(terminal: &mut Terminal<B>, tui: &mut TuiClap) -> io::Result<()>{
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
        let inset_area = edge_inset(&chunks[1], 1);
        tui.render_output(f, inset_area);
        let block = Block::default()
            .title("Command")
            .borders(Borders::ALL);
        f.render_widget(block, chunks[2]);

        let inset_area = edge_inset(&chunks[2], 1);
        tui.render_input(f, inset_area);
    })?;
    Ok(())
}

fn edge_inset(area: &Rect, margin: u16) -> Rect {
    let mut inset_area = area.clone();
    inset_area.x += margin;
    inset_area.y += margin;
    inset_area.height -= margin;
    inset_area.width -= margin;

    inset_area
}

fn handle_matches(matches: ArgMatches) -> Result<Vec<String>, String> {
    let mut output = vec![];
    
    let config = matches.value_of("config").unwrap_or("default.conf");
    let out = format!("Value for config: {}", config);
    output.push(out);

    // Calling .unwrap() is safe here because "INPUT" is required (if "INPUT" wasn't
    // required we could have used an 'if let' to conditionally get the value)
    let out = format!("Using input file: {}", matches.value_of("INPUT").unwrap());
    output.push(out);

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    let out = match matches.occurrences_of("v") {
        0 => format!("No verbose info"),
        1 => format!("Some verbose info"),
        2 => format!("Tons of verbose info"),
        3 | _ => format!("Don't be crazy"),
    };
    output.push(out);

    // You can handle information about subcommands by requesting their matches by name
    // (as below), requesting just the name used, or both at the same time
    if let Some(matches) = matches.subcommand_matches("test") {
        if matches.is_present("debug") {
            let out = format!("Printing debug info...");
            output.push(out);
        } else {
            let out = format!("Printing normally...");
            output.push(out);
        }
    };

    Ok(output)
}