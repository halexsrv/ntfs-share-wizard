use std::io::{self, Stdout};

use anyhow::Result;
use crossterm::event::{self, Event};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{ExecutableCommand, execute};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn render_once(app: &App, title: &str, details: &str) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|frame| {
        let area = frame.area();
        let sections = Layout::vertical([Constraint::Length(3), Constraint::Min(5)]).split(area);

        let header = Paragraph::new(Line::from(title))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("ntfs-share-wizard"),
            )
            .style(Style::default().add_modifier(Modifier::BOLD));

        let body = Paragraph::new(Text::from(format!(
            "OS: {}\n\n{}",
            app.operating_system().display_name(),
            details
        )))
        .block(Block::default().borders(Borders::ALL).title("status"));

        frame.render_widget(header, sections[0]);
        frame.render_widget(body, sections[1]);
    })?;

    let _ = event::read().or_else(ignore_missing_tty_event);
    restore_terminal(&mut terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn ignore_missing_tty_event(error: io::Error) -> io::Result<Event> {
    if error.kind() == io::ErrorKind::WouldBlock {
        Ok(Event::Resize(0, 0))
    } else {
        Err(error)
    }
}
