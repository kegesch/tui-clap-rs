# tui-clap
Input widgets are not supported by tui-rs out of the box. This crate provides an abstraction of input handling together with clap's command argument parsing. 

# Getting Started
`tui-clap` is providing two widgets (input and output) and takes care of parsing the input against a `clap` app. 
To get it work three points must be implemented manually: 
* fetching events must be included in the main loop
* output and input widgets must be rendered
* arg matches from clap must be handled

The following code demonstrates these three points.

```rust
fn main() -> Result<(), io::Error> {
    let yaml = load_yaml!("cli.yaml");
    let clapp = App::from(yaml);

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create a TuiClap instance and pass over a function that handles the arg matches
    let mut tui = TuiClap::from_app(clapp);
    
    terminal.clear();
    
    // handle events, Events struct is a helper struct to read from crossterm events
    let events = Events::default();
    
    loop {
        // your drawing method
        draw(&mut terminal, &mut tui)?;
        
        // handle events manually with the provided events struct, but you can use your own
        if let Ok(Some(Event::Key(key_event))) = events.next() {
            match key_event.code {
                KeyCode::Backspace => {
                    tui.state().del_char()
                }
                KeyCode::Enter => {
                    if let Ok(matches) = tui.parse() {
                        match handle_matches(matches) {
                            Ok(output) => {
                                for message in output {
                                    tui.write_to_output(message)
                                }
                            }
                            Err(err) => tui.write_to_output(err)
                        }
                    }
                }
                KeyCode::Char(char) => {
                    tui.state().add_char(char)
                },
                _ => {}
            }
        }
    }
}

// your drawing method
fn draw<B: Backend>(terminal: &mut Terminal<B>, tui: &mut TuiClap) -> io::Result<()>{
    terminal.draw(|f| {
        let size = f.size();
        // render the input widget of tui-clap
        tui.render_input(f, size);
        
        // render the output widget of tui-clap
        tui.render_output(f, size);
    });
    
    Ok(())
}

// function that handles arg matches and returns a vec of strings that is pushed to the output widget
// return Ok() with vec of message that should be added to the output
// return Err(message) to display an error in the output
fn handle_matches(matches: ArgMatches) -> Result<Vec<String>, String> {}
    Ok(vec!["handled".to_string()])
}
```

# Example
See the `example` folder or run `cargo run --example command`
