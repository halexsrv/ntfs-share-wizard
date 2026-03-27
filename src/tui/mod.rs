use std::io::{self, Stdout};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
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

use crate::app::{App, Screen};
use crate::os::OperatingSystem;

pub fn run(mut app: App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let run_result = run_event_loop(&mut terminal, &mut app);
    let cleanup_result = restore_terminal(&mut terminal);

    run_result?;
    cleanup_result
}

fn run_event_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    while !app.should_quit() {
        let view = current_view(app);

        terminal.draw(|frame| {
            let area = frame.area();
            let sections =
                Layout::vertical([Constraint::Length(3), Constraint::Min(5)]).split(area);

            let header = Paragraph::new(Line::from(view.title))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("ntfs-share-wizard"),
                )
                .style(Style::default().add_modifier(Modifier::BOLD));

            let body = Paragraph::new(Text::from(format!(
                "OS: {}\n\n{}\n\n{}",
                app.operating_system().display_name(),
                view.body,
                key_hints(app)
            )))
            .block(Block::default().borders(Borders::ALL).title("status"));

            frame.render_widget(header, sections[0]);
            frame.render_widget(body, sections[1]);
        })?;

        if let Event::Key(key_event) = event::read().or_else(ignore_missing_tty_event)? {
            if key_event.kind != KeyEventKind::Press {
                continue;
            }

            match key_event.code {
                KeyCode::Char('q') => app.request_quit(),
                KeyCode::Enter => app.advance(),
                KeyCode::Esc => app.go_back(),
                _ => {}
            }
        }
    }

    Ok(())
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

pub struct View<'a> {
    pub title: &'a str,
    pub body: String,
}

fn current_view(app: &App) -> View<'static> {
    match app.current_screen() {
        Screen::Welcome => View {
            title: "Welcome",
            body: "Press Enter to continue to the detected system flow.".to_owned(),
        },
        Screen::DetectedSystem => detected_system_view(app),
        Screen::WindowsWizard => crate::windows::wizard::current_view(app),
        Screen::Unsupported => unsupported_view(app),
    }
}

fn detected_system_view(app: &App) -> View<'static> {
    match app.operating_system() {
        OperatingSystem::Windows => View {
            title: "Detected System",
            body: crate::windows::wizard::detected_system_details(app),
        },
        OperatingSystem::Linux(_) => View {
            title: "Detected System",
            body: crate::linux::wizard::detected_system_details(app),
        },
        OperatingSystem::Unsupported(name) => View {
            title: "Unsupported",
            body: format!("Unsupported operating system detected: {name}"),
        },
    }
}

fn unsupported_view(app: &App) -> View<'static> {
    let details = match app.operating_system() {
        OperatingSystem::Unsupported(name) => {
            format!("Unsupported operating system detected: {name}")
        }
        supported => format!(
            "Unsupported screen reached unexpectedly for supported OS: {}",
            supported.display_name()
        ),
    };

    View {
        title: "Unsupported",
        body: details,
    }
}

fn key_hints(app: &App) -> &'static str {
    let screen = app.current_screen();
    match screen {
        Screen::Welcome => "Enter: advance | q: quit",
        Screen::WindowsWizard => crate::windows::wizard::key_hints(app.windows_wizard()),
        Screen::DetectedSystem | Screen::Unsupported => {
            "Enter: keep current screen | Esc: back | q: quit"
        }
    }
}
