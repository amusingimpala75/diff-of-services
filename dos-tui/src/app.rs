use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode},
    widgets::{Block, Paragraph, Widget},
};

use anyhow::Result;

pub(crate) struct App {
    exit: bool,
}

impl App {
    pub(crate) fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        terminal.draw(|frame| self.draw(frame))?;
        self.handle_event()?;
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_event(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => self.exit = true,
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn default() -> Self {
        App { exit: false }
    }
}

impl Widget for &App {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let block = Block::bordered().title("Diff of Services");
        Paragraph::new("Hello, world!\nPress 'q' to exit.")
            .centered()
            .block(block)
            .render(area, buf);
    }
}
