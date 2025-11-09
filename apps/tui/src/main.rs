use std::{
    collections::HashMap,
    io,
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute, terminal,
};
use hn_core::{HackerNewsClient, HackerNewsComment, HackerNewsItem};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

const COMMENT_LIMIT: usize = 10;
const POLL_INTERVAL: Duration = Duration::from_millis(150);

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

    let app = App::new(posts);
    let res = run_app(&mut terminal, app, client);

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), terminal::LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    client: HackerNewsClient,
) -> color_eyre::Result<()> {
    let (tx, rx) = mpsc::channel::<CommentFetchMessage>();
    app.ensure_comments_for_selection(&client, &tx);

    loop {
        drain_comment_messages(&mut app, &rx);

        terminal.draw(|frame| draw_ui(frame, &mut app))?;

        if event::poll(POLL_INTERVAL)? {
            if let Event::Key(key) = event::read()? {
                let (should_quit, selection_changed) = app.handle_key_event(key);
                if should_quit {
                    break;
                }
                if selection_changed {
                    app.ensure_comments_for_selection(&client, &tx);
                }
            }
        }
    }

    Ok(())
}

fn drain_comment_messages(app: &mut App, rx: &Receiver<CommentFetchMessage>) {
    while let Ok(message) = rx.try_recv() {
        app.process_comment_message(message);
    }
}

fn draw_ui(frame: &mut Frame, app: &mut App) {
    let root_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(frame.size());

    let header = Paragraph::new("Hacker News — Shared Rust WebAssembly")
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    frame.render_widget(header, root_chunks[0]);

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(root_chunks[1]);

    render_posts(frame, body_chunks[0], app);
    render_comments(frame, body_chunks[1], app);
}

fn render_posts(frame: &mut Frame, area: ratatui::layout::Rect, app: &mut App) {
    let items: Vec<ListItem> = app
        .posts
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let title = format!("{:>2}. {}", idx + 1, item.title);
            let meta = format!("{} points • {}", item.score, item.by);
            ListItem::new(vec![
                Line::from(title),
                Line::from(Span::styled(meta, Style::default().fg(Color::Gray))),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title("Top Stories").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, app.list_state());
}

fn render_comments(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let title = app
        .selected_post()
        .map(|post| format!("Comments — {}", post.title))
        .unwrap_or_else(|| "Comments".to_string());

    match app.comment_status {
        CommentStatus::Loading => {
            let paragraph = Paragraph::new("Loading comments…")
                .block(comments_block(&title))
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
        }
        CommentStatus::Error(ref msg) => {
            let paragraph = Paragraph::new(format!("Failed to load comments:\n{msg}"))
                .style(Style::default().fg(Color::Red))
                .block(comments_block(&title))
                .wrap(Wrap { trim: true });
            frame.render_widget(paragraph, area);
        }
        CommentStatus::Ready => {
            if let Some(comments) = app.comments_for_selected() {
                if comments.is_empty() {
                    let paragraph = Paragraph::new("No comments yet.")
                        .block(comments_block(&title))
                        .alignment(Alignment::Center);
                    frame.render_widget(paragraph, area);
                } else {
                    let items: Vec<ListItem> = comments
                        .iter()
                        .map(|comment| {
                            let header = format!("{} (#{})", comment.by, comment.id);
                            let text = sanitize_comment_text(&comment.text);
                            ListItem::new(vec![
                                Line::from(Span::styled(
                                    header,
                                    Style::default()
                                        .fg(Color::Cyan)
                                        .add_modifier(Modifier::BOLD),
                                )),
                                Line::from(text),
                                Line::from(""),
                            ])
                        })
                        .collect();

                    let comment_list = List::new(items)
                        .block(comments_block(&title))
                        .highlight_symbol("");
                    frame.render_widget(comment_list, area);
                }
            } else {
                let paragraph = Paragraph::new("Select a post to view comments.")
                    .block(comments_block(&title))
                    .alignment(Alignment::Center);
                frame.render_widget(paragraph, area);
            }
        }
        CommentStatus::Idle => {
            let paragraph = Paragraph::new("Use ↑/↓ or j/k to navigate posts.")
                .block(comments_block(&title))
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
        }
    }
}

struct App {
    posts: Vec<HackerNewsItem>,
    list_state: ListState,
    comments_cache: HashMap<u64, Vec<HackerNewsComment>>,
    inflight_story: Option<u64>,
    comment_status: CommentStatus,
}

impl App {
    fn new(posts: Vec<HackerNewsItem>) -> Self {
        let mut list_state = ListState::default();
        if !posts.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            posts,
            list_state,
            comments_cache: HashMap::new(),
            inflight_story: None,
            comment_status: CommentStatus::Idle,
        }
    }

    fn list_state(&mut self) -> &mut ListState {
        &mut self.list_state
    }

    fn selected_post(&self) -> Option<&HackerNewsItem> {
        self.list_state
            .selected()
            .and_then(|idx| self.posts.get(idx))
    }

    fn comments_for_selected(&self) -> Option<&Vec<HackerNewsComment>> {
        let post = self.selected_post()?;
        self.comments_cache.get(&post.id)
    }

    fn ensure_comments_for_selection(
        &mut self,
        client: &HackerNewsClient,
        tx: &Sender<CommentFetchMessage>,
    ) {
        if let Some(post) = self.selected_post().cloned() {
            if self.comments_cache.contains_key(&post.id) {
                self.comment_status = CommentStatus::Ready;
                self.inflight_story = None;
            } else if self.inflight_story == Some(post.id) {
                self.comment_status = CommentStatus::Loading;
            } else {
                self.comment_status = CommentStatus::Loading;
                self.inflight_story = Some(post.id);

                let tx = tx.clone();
                let client = client.clone();
                tokio::spawn(async move {
                    let result = client
                        .fetch_comments_for(&post, COMMENT_LIMIT)
                        .await
                        .map_err(|err| err.to_string());

                    let _ = tx.send(CommentFetchMessage {
                        story_id: post.id,
                        result,
                    });
                });
            }
        } else {
            self.inflight_story = None;
            self.comment_status = CommentStatus::Idle;
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> (bool, bool) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => (true, false),
            KeyCode::Down | KeyCode::Char('j') => (false, self.move_selection(1)),
            KeyCode::Up | KeyCode::Char('k') => (false, self.move_selection(-1)),
            _ => (false, false),
        }
    }

    fn move_selection(&mut self, delta: isize) -> bool {
        if self.posts.is_empty() {
            return false;
        }

        let len = self.posts.len() as isize;
        let current = self.list_state.selected().unwrap_or(0) as isize;
        let mut next = current + delta;
        if next < 0 {
            next = len - 1;
        } else if next >= len {
            next = 0;
        }

        let next = next as usize;
        if self.list_state.selected() == Some(next) {
            return false;
        }

        self.list_state.select(Some(next));
        true
    }

    fn selected_post_id(&self) -> Option<u64> {
        self.selected_post().map(|post| post.id)
    }

    fn process_comment_message(&mut self, message: CommentFetchMessage) {
        if self.inflight_story == Some(message.story_id) {
            self.inflight_story = None;
        }

        match message.result {
            Ok(comments) => {
                self.comments_cache.insert(message.story_id, comments);
                if self.selected_post_id() == Some(message.story_id) {
                    self.comment_status = CommentStatus::Ready;
                }
            }
            Err(err) => {
                if self.selected_post_id() == Some(message.story_id) {
                    self.comment_status = CommentStatus::Error(err);
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CommentStatus {
    Idle,
    Loading,
    Ready,
    Error(String),
}

fn comments_block(title: &str) -> Block<'static> {
    Block::default()
        .title(title.to_string())
        .borders(Borders::ALL)
}

struct CommentFetchMessage {
    story_id: u64,
    result: Result<Vec<HackerNewsComment>, String>,
}

fn sanitize_comment_text(input: &str) -> String {
    let replacements = [
        ("<p>", "\n\n"),
        ("</p>", ""),
        ("<i>", ""),
        ("</i>", ""),
        ("<em>", ""),
        ("</em>", ""),
        ("<strong>", ""),
        ("</strong>", ""),
        ("<code>", "`"),
        ("</code>", "`"),
        ("<pre>", "\n"),
        ("</pre>", "\n"),
        ("<br>", "\n"),
        ("<br/>", "\n"),
        ("<br />", "\n"),
        ("&gt;", ">"),
        ("&lt;", "<"),
        ("&amp;", "&"),
        ("&quot;", "\""),
        ("&#x27;", "'"),
        ("&#x2F;", "/"),
        ("&nbsp;", " "),
    ];

    let mut output = input.to_string();
    for (from, to) in replacements {
        output = output.replace(from, to);
    }

    strip_tags(&output).trim().to_string()
}

fn strip_tags(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => {
                in_tag = true;
            }
            '>' => {
                in_tag = false;
            }
            _ => {
                if !in_tag {
                    result.push(ch);
                }
            }
        }
    }

    result
}
