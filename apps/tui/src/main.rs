use std::io;

use crossterm::{event, execute, terminal};
use hn_core::{HackerNewsClient, HackerNewsItem};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let client = HackerNewsClient::default();
    let posts = client.fetch_top_stories(20).await?;

    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, posts);

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), terminal::LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    posts: Vec<HackerNewsItem>,
) -> color_eyre::Result<()> {
    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
                .split(frame.size());

            let header = Paragraph::new("Hacker News — Shared Rust WebAssembly").style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_widget(header, chunks[0]);

            let items: Vec<ListItem> = posts
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    format!(
                        "{idx:>2}. {title} ({score} points) — {by}",
                        title = item.title,
                        score = item.score,
                        by = item.by
                    )
                })
                .map(|text| ListItem::new(Line::from(text)))
                .collect();

            let list =
                List::new(items).block(Block::default().title("Top Stories").borders(Borders::ALL));
            frame.render_widget(list, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(250))? {
            if let event::Event::Key(key) = event::read()? {
                if key.code == event::KeyCode::Char('q') || key.code == event::KeyCode::Esc {
                    break;
                }
            }
        }
    }

    Ok(())
}
