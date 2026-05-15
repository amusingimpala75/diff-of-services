use dos_lib::{document::Document, revision::Revision};
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyModifiers},
    layout::{Constraint, Layout},
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, List, Paragraph, Widget, Wrap},
};

use anyhow::Result;

pub(crate) struct App {
    exit: bool,
    documents: Vec<Document>,
    selected_document_idx: usize,
    revisions: Vec<Revision>,
    selected_revision_idx: usize,
    selected_revision: Option<Revision>,
    previous_revision: Option<Revision>,
    vertical_scroll: u16,
}

impl App {
    pub(crate) fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_event()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn set_selected_document(&mut self, idx: usize) -> Result<()> {
        let idx = idx.clamp(0, self.documents.len());
        self.selected_document_idx = idx;
        self.revisions = self.documents[self.selected_document_idx]
            .revisions(&dos_lib::open_connection_file()?)?;
        self.revisions.reverse();
        self.set_selected_revision(0)?;
        Ok(())
    }

    fn set_selected_revision(&mut self, idx: usize) -> Result<()> {
        let idx = idx.clamp(0, self.revisions.len());
        self.selected_revision_idx = idx;
        self.vertical_scroll = 0;
        if self.selected_revision_idx < self.revisions.len() {
            self.selected_revision = Some(
                self.revisions[self.selected_revision_idx]
                    .clone()
                    .load_text(&dos_lib::open_connection_file()?)?,
            );
            if self.selected_revision_idx + 1 < self.revisions.len() {
                self.previous_revision = Some(
                    self.revisions[self.selected_revision_idx + 1]
                        .clone()
                        .load_text(&dos_lib::open_connection_file()?)?,
                );
            }
        } else {
            self.selected_revision = None;
        }
        Ok(())
    }

    fn handle_event(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => self.exit = true,
                KeyCode::Up => {
                    if key.modifiers.contains(KeyModifiers::ALT) {
                        if self.selected_document_idx > 0 {
                            self.set_selected_document(self.selected_document_idx - 1)?;
                        }
                    } else if key.modifiers.contains(KeyModifiers::SHIFT) {
                        if self.selected_revision_idx > 0 {
                            self.set_selected_revision(self.selected_revision_idx - 1)?;
                        }
                    } else if self.vertical_scroll > 0 {
                        self.vertical_scroll -= 1;
                    }
                }
                KeyCode::Down => {
                    if key.modifiers.contains(KeyModifiers::ALT) {
                        if self.selected_document_idx + 1 < self.documents.len() {
                            self.set_selected_document(self.selected_document_idx + 1)?;
                        }
                    } else if key.modifiers.contains(KeyModifiers::SHIFT) {
                        if self.selected_revision_idx + 1 < self.revisions.len() {
                            self.set_selected_revision(self.selected_revision_idx + 1)?;
                        }
                    } else {
                        self.vertical_scroll += 1;
                    }
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn default() -> Result<Self> {
        let conn = dos_lib::open_connection_file()?;
        let documents = Document::get_all(&conn)?;
        let mut app = App {
            exit: false,
            documents,
            selected_document_idx: 0,
            revisions: vec![],
            selected_revision_idx: 0,
            selected_revision: None,
            previous_revision: None,
            vertical_scroll: 0,
        };

        app.set_selected_document(0)?;

        Ok(app)
    }
}

impl Widget for &App {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let title = Line::from(" Diff of Services ");
        let nav = Line::from(
            " M-Down / M-Up to change documents, S-Up / S-Down to change revisions, Up / Down to scroll the document ",
        );

        let generic_border = Block::bordered().border_set(border::THICK);

        let main_border = generic_border
            .clone()
            .title(title.centered())
            .title_bottom(nav);

        (&main_border).render(area, buf);

        let [docu_select, rev_select, docu_display] = Layout::horizontal([
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(70),
        ])
        .areas(main_border.inner(area));

        Widget::render(
            List::new(self.documents.iter().enumerate().map(|(idx, doc)| {
                if idx == self.selected_document_idx {
                    doc.name().reversed()
                } else {
                    doc.name().into()
                }
            }))
            .block(generic_border.clone())
            .highlight_style(Style::new().reversed())
            .highlight_symbol(">")
            .repeat_highlight_symbol(true),
            docu_select,
            buf,
        );

        Widget::render(
            List::new(self.revisions.iter().enumerate().map(|(idx, rev)| {
                let date = dos_lib::format_local_time(rev.added_on());
                if idx == self.selected_revision_idx {
                    format!("{date}").reversed()
                } else {
                    format!("{date}").into()
                }
            }))
            .block(generic_border.clone())
            .highlight_symbol(">")
            .highlight_style(Style::new().reversed())
            .repeat_highlight_symbol(true),
            rev_select,
            buf,
        );

        (&generic_border).render(docu_display, buf);

        if let Some(current_text) = self.selected_revision.clone().map(|rev| rev.text.unwrap()) {
            if let Some(previous_text) = self.previous_revision.clone().map(|rev| rev.text.unwrap())
            {
                Paragraph::new(
                    diff::lines(&previous_text, &current_text)
                        .iter()
                        .map(|res| match res {
                            diff::Result::Left(l) => Line::from(l.to_string().red()),
                            diff::Result::Both(l, _) => Line::from(l.to_string()),
                            diff::Result::Right(r) => Line::from(r.to_string().green()),
                        })
                        .collect::<Vec<_>>(),
                )
            } else {
                Paragraph::new(current_text)
            }
                .wrap(Wrap { trim: false })
                .scroll((self.vertical_scroll * 5, 0))
                .render(generic_border.inner(docu_display), buf);
        }
    }
}
