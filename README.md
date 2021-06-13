# tui-clap
Input widgets are not supported by tui-rs out of the box. This crate provides an abstraction of input handling together with clap's command argument parsing. 

# Example

```rust 
fn main() -> Result<(), io::Error> {

    let app = App::new("My Super Program")
        .setting(AppSettings::NoBinaryName)
        .version("1.0")
        .author("Kevin K. <kbknapp@gmail.com>")
        .about("Does awesome things")
        .arg(Arg::new("config")
            .short('c')
            .long("config")
            .value_name("FILE")
            .about("Sets a custom config file")
            .takes_value(true))
        .arg(Arg::new("INPUT")
            .about("Sets the input file to use")
            .required(true)
            .index(1))
        .arg(Arg::new("v")
            .short('v')
            .multiple(true)
            .takes_value(true)
            .about("Sets the level of verbosity"))
        .subcommand(App::new("test")
            .about("controls testing features")
            .version("1.3")
            .author("Someone E. <someone_else@other.com>")
            .arg(Arg::new("debug")
                .short('d')
                .about("print debug information verbosely")));

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut tui = TuiClap::from_app(app);
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
        let block = Block::default()
            .title("Command")
            .borders(Borders::ALL);
        f.render_widget(block, chunks[2]);
        tui.render_input(f, chunks[2]);
    })?;
    Ok(())
}

```
