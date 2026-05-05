use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    widgets::{Block, Paragraph},
};

fn main() -> anyhow::Result<()> {
    ratatui::run(|terminal| {
        let mut exit = false;
        while !exit {
            terminal.draw(|frame| {
                let block = Block::bordered().title("Diff of Services");
                let greeting = Paragraph::new("Hello, world!\nPress 'q' to exit.")
                    .centered()
                    .block(block);
                frame.render_widget(greeting, frame.area());
            })?;

            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') => exit = true,
                    _ => {}
                },
                _ => {}
            }
        }

        Ok(())
    })
}
