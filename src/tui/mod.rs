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
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
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
            let sections = Layout::vertical([
                Constraint::Length(4),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(area);

            let header = Paragraph::new(Text::from(vec![
                Line::from(vec![
                    Span::styled(
                        "ntfs-share-wizard",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(view.title, Style::default().add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![Span::styled(
                    screen_subtitle(app),
                    Style::default().fg(Color::Gray),
                )]),
            ]))
            .block(Block::default().borders(Borders::ALL).title("Wizard"));

            let body = Paragraph::new(format_body_text(app, &view.body))
                .block(Block::default().borders(Borders::ALL).title("Detalhes"));

            let footer = Paragraph::new(Line::from(vec![
                Span::styled("Teclas: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(key_hints(app)),
            ]))
            .block(Block::default().borders(Borders::ALL).title("Ajuda"));

            frame.render_widget(header, sections[0]);
            frame.render_widget(body, sections[1]);
            frame.render_widget(footer, sections[2]);
        })?;

        if let Event::Key(key_event) = event::read().or_else(ignore_missing_tty_event)? {
            if key_event.kind != KeyEventKind::Press {
                continue;
            }

            match key_event.code {
                KeyCode::Char('q') => app.request_quit(),
                KeyCode::Enter => app.advance(),
                KeyCode::Esc => app.go_back(),
                KeyCode::Up => app.move_selection_up(),
                KeyCode::Down => app.move_selection_down(),
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
            title: "Inicio",
            body: "[INFO] Bem-vindo ao ntfs-share-wizard.\n\nEste assistente ajuda a preparar o compartilhamento seguro de uma biblioteca Steam em NTFS entre Windows e Linux.\n\nPressione Enter para continuar.".to_owned(),
        },
        Screen::DetectedSystem => detected_system_view(app),
        Screen::LinuxWizard => crate::linux::wizard::current_view(app),
        Screen::WindowsWizard => crate::windows::wizard::current_view(app),
        Screen::Unsupported => unsupported_view(app),
    }
}

fn detected_system_view(app: &App) -> View<'static> {
    match app.operating_system() {
        OperatingSystem::Windows => View {
            title: "Sistema Detectado",
            body: crate::windows::wizard::detected_system_details(app),
        },
        OperatingSystem::Linux(_) => View {
            title: "Sistema Detectado",
            body: crate::linux::wizard::detected_system_details(app),
        },
        OperatingSystem::Unsupported(name) => View {
            title: "Nao Suportado",
            body: format!("[ERROR] Sistema operacional nao suportado detectado: {name}"),
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
        title: "Nao Suportado",
        body: format!("[ERROR] {details}"),
    }
}

fn key_hints(app: &App) -> &'static str {
    let screen = app.current_screen();
    match screen {
        Screen::Welcome => "Enter confirmar | q sair",
        Screen::LinuxWizard => crate::linux::wizard::key_hints(app.linux_wizard()),
        Screen::WindowsWizard => crate::windows::wizard::key_hints(app.windows_wizard()),
        Screen::DetectedSystem | Screen::Unsupported => "Enter confirmar | Esc voltar | q sair",
    }
}

fn screen_subtitle(app: &App) -> String {
    match app.current_screen() {
        Screen::Welcome => "Preparacao inicial do wizard".to_owned(),
        Screen::DetectedSystem => format!(
            "Sistema detectado: {}",
            app.operating_system().display_name()
        ),
        Screen::LinuxWizard => "Fluxo Linux com validacoes e configuracao guiada".to_owned(),
        Screen::WindowsWizard => "Fluxo Windows para desabilitar Fast Startup".to_owned(),
        Screen::Unsupported => "Sistema atual fora do escopo suportado".to_owned(),
    }
}

fn format_body_text(app: &App, body: &str) -> Text<'static> {
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Sistema: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(app.operating_system().display_name().to_owned()),
        ]),
        Line::default(),
    ];

    for line in body.lines() {
        lines.push(format_status_line(line));
    }

    Text::from(lines)
}

fn format_status_line(line: &str) -> Line<'static> {
    if let Some(rest) = line.strip_prefix("[SUCCESS] ") {
        return Line::from(vec![
            Span::styled(
                "[SUCCESS] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(rest.to_owned()),
        ]);
    }

    if let Some(rest) = line.strip_prefix("[WARNING] ") {
        return Line::from(vec![
            Span::styled(
                "[WARNING] ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(rest.to_owned()),
        ]);
    }

    if let Some(rest) = line.strip_prefix("[ERROR] ") {
        return Line::from(vec![
            Span::styled(
                "[ERROR] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(rest.to_owned()),
        ]);
    }

    if let Some(rest) = line.strip_prefix("[INFO] ") {
        return Line::from(vec![
            Span::styled(
                "[INFO] ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(rest.to_owned()),
        ]);
    }

    Line::from(line.to_owned())
}
