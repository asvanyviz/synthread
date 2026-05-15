//! Synthread headless binary — TUI (Ratatui) + embedded WebUI.
//!
//! Usage:
//!   synthread                     # auto-detect mode
//!   synthread --mode tui          # force TUI
//!   synthread --mode headless     # headless (WebUI + API only)

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame,
};
use std::io;
use synthread_api_client::ApiClient;

#[derive(Parser)]
#[command(
    name = "synthread",
    version,
    about = "P2P framework with plugin system"
)]
struct Args {
    #[arg(long, default_value = "auto")]
    mode: String,
    #[arg(long, default_value = "7700")]
    port: u16,
}

// ── TUI State ──

enum View {
    Peers,
    Chat,
    Status,
}

struct TuiState {
    api: ApiClient,
    current_view: View,
    peer_list: Vec<synthread_api_client::PeerInfo>,
    selected_peer: usize,
    chat_messages: Vec<String>,
    input: String,
    status_line: String,
    connected: bool,
    peer_id: String,
}

impl TuiState {
    fn new(api: ApiClient) -> Self {
        Self {
            api,
            current_view: View::Peers,
            peer_list: Vec::new(),
            selected_peer: 0,
            chat_messages: Vec::new(),
            input: String::new(),
            status_line: "Connecting...".to_string(),
            connected: false,
            peer_id: "unknown".to_string(),
        }
    }

    async fn refresh_peers(&mut self) {
        match self.api.list_peers().await {
            Ok(peers) => {
                self.peer_list = peers;
                self.status_line = format!("{} peers", self.peer_list.len());
            }
            Err(e) => {
                self.status_line = format!("Error: {}", e);
            }
        }
    }

    async fn refresh_chat(&mut self) {
        if self.peer_list.is_empty() {
            return;
        }
        let peer = &self.peer_list[self
            .selected_peer
            .min(self.peer_list.len().saturating_sub(1))];
        match self.api.chat_messages(&peer.peer_id, None, 50).await {
            Ok(msgs) => {
                self.chat_messages = serde_json::from_value(msgs).unwrap_or_default();
                // Format as strings
                self.chat_messages = self
                    .chat_messages
                    .iter()
                    .map(|m| format!("{}", m))
                    .collect();
            }
            Err(e) => {
                self.chat_messages = vec![format!("Error: {}", e)];
            }
        }
    }

    async fn check_connection(&mut self) {
        match self.api.status().await {
            Ok(s) => {
                self.connected = true;
                self.peer_id = s.peer_id.clone();
                self.status_line = format!(
                    "{} | peers: {}/{} | friends: {}",
                    s.peer_id, s.connected_peers, s.known_peers, s.friends
                );
            }
            Err(_) => {
                self.connected = false;
                self.status_line = "Not connected — is synthread running?".to_string();
            }
        }
    }
}

// ── TUI App ──

struct App {
    state: TuiState,
    should_quit: bool,
}

impl App {
    fn new(api: ApiClient) -> Self {
        Self {
            state: TuiState::new(api),
            should_quit: false,
        }
    }

    async fn on_tick(&mut self) {
        self.state.check_connection().await;
        self.state.refresh_peers().await;
    }

    fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('1') => self.state.current_view = View::Peers,
            KeyCode::Char('2') => self.state.current_view = View::Chat,
            KeyCode::Char('3') => self.state.current_view = View::Status,
            KeyCode::Tab => {
                self.state.current_view = match self.state.current_view {
                    View::Peers => View::Chat,
                    View::Chat => View::Status,
                    View::Status => View::Peers,
                };
            }
            KeyCode::Up => {
                if self.state.selected_peer > 0 {
                    self.state.selected_peer -= 1;
                }
            }
            KeyCode::Down => {
                if self.state.selected_peer + 1 < self.state.peer_list.len() {
                    self.state.selected_peer += 1;
                }
            }
            KeyCode::Char(c) => {
                self.state.input.push(c);
            }
            KeyCode::Backspace => {
                self.state.input.pop();
            }
            KeyCode::Enter => {
                if !self.state.input.is_empty() {
                    // TODO: send through API
                    self.state
                        .chat_messages
                        .push(format!("> {}", self.state.input));
                    self.state.input.clear();
                }
            }
            _ => {}
        }
    }

    fn ui(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // tabs
                Constraint::Min(1),    // content
                Constraint::Length(3), // input/status
            ])
            .split(f.area());

        // Tabs
        let tabs = Tabs::new(vec!["1:Peers", "2:Chat", "3:Status"])
            .select(match self.state.current_view {
                View::Peers => 0,
                View::Chat => 1,
                View::Status => 2,
            })
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_widget(tabs, chunks[0]);

        match self.state.current_view {
            View::Peers => self.render_peers(f, chunks[1]),
            View::Chat => self.render_chat(f, chunks[1]),
            View::Status => self.render_status(f, chunks[1]),
        }

        // Input bar
        let input = Paragraph::new(self.state.input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Input"))
            .style(Style::default().fg(Color::White));
        f.render_widget(input, chunks[2]);
    }

    fn render_peers(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .state
            .peer_list
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let style = if i == self.state.selected_peer {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let rel = match p.relationship.as_str() {
                    "friend" => "👤",
                    "known" => "•",
                    _ => "?",
                };
                let prio = if p.priority { " ⭐" } else { "" };
                ListItem::new(Line::from(Span::styled(
                    format!(
                        "{} {} {} ({}){}{}",
                        i + 1,
                        rel,
                        &p.peer_id[..p.peer_id.len().min(16)],
                        p.connection,
                        prio,
                        if let Some(ms) = p.latency_ms {
                            format!(" {}ms", ms)
                        } else {
                            String::new()
                        },
                    ),
                    style,
                )))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Peers"))
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow));
        f.render_widget(list, area);
    }

    fn render_chat(&self, f: &mut Frame, area: Rect) {
        let peer_name = if self.state.peer_list.is_empty() {
            "No peers"
        } else {
            let idx = self.state.selected_peer.min(self.state.peer_list.len() - 1);
            &self.state.peer_list[idx].peer_id
        };

        let messages: Vec<Line> = self
            .state
            .chat_messages
            .iter()
            .map(|m| Line::from(Span::styled(m, Style::default().fg(Color::White))))
            .collect();

        let chat = Paragraph::new(messages)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Chat — {}", peer_name)),
            )
            .scroll((self.state.chat_messages.len().saturating_sub(10) as u16, 0));
        f.render_widget(chat, area);
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        let status_text = vec![
            Line::from(Span::styled(
                format!("Peer ID: {}", self.state.peer_id),
                Style::default().fg(Color::Cyan),
            )),
            Line::from(Span::styled(
                format!("Connected: {}", self.state.connected),
                Style::default().fg(if self.state.connected {
                    Color::Green
                } else {
                    Color::Red
                }),
            )),
            Line::from(Span::styled(
                format!("Peers known: {}", self.state.peer_list.len()),
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Controls: 1/2/3 switch view | Tab cycle | ↑↓ navigate | Enter send | q quit",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let status = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status, area);
    }
}

// ── Main ──

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    tracing::info!(
        "Synthread v{} starting in {} mode",
        env!("CARGO_PKG_VERSION"),
        args.mode
    );

    let api = ApiClient::new(&format!("http://127.0.0.1:{}", args.port));

    match args.mode.as_str() {
        "headless" => {
            tracing::info!("Starting headless mode on port {}", args.port);
            match synthread_core::node::NodeBuilder::new(Default::default()).build() {
                Ok(mut node) => {
                    // Start listening on default address
                    let addr: libp2p::Multiaddr = "/ip4/0.0.0.0/tcp/9000".parse().unwrap();
                    if let Err(e) = node.start_listening(&[addr]) {
                        tracing::error!("Failed to start listening: {}", e);
                        return Ok(());
                    }

                    // Clone API server before node gets moved
                    let api = node.api.clone();

                    // Spawn API server
                    let api_handle = tokio::spawn(async move {
                        if let Err(e) = api.start(args.port).await {
                            tracing::error!("API server error: {}", e);
                        }
                    });

                    // Spawn event loop (moves node)
                    let event_handle = tokio::spawn(async move {
                        node.run_event_loop().await;
                    });

                    tracing::info!("Synthread node running at http://127.0.0.1:{}", args.port);

                    tokio::select! {
                        _ = tokio::signal::ctrl_c() => {
                            tracing::info!("Shutting down...");
                        }
                        _ = api_handle => {}
                        _ = event_handle => {}
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create node: {}", e);
                }
            }
        }
        "tui" | _ => {
            run_tui(api).await?;
        }
    }

    Ok(())
}

async fn run_tui(api: ApiClient) -> io::Result<()> {
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let mut app = App::new(api);

    // Initial refresh
    app.on_tick().await;

    // Main loop
    let tick_rate = std::time::Duration::from_secs(5);
    let mut last_tick = std::time::Instant::now();

    loop {
        // Render
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(io::stdout()))?;
        terminal.draw(|f| app.ui(f))?;

        // Poll for input with timeout
        if event::poll(
            tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or(std::time::Duration::from_millis(100)),
        )? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key.code);
                }
            }
        }

        if app.should_quit {
            break;
        }

        // Tick
        if last_tick.elapsed() >= tick_rate {
            app.on_tick().await;
            last_tick = std::time::Instant::now();
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

    Ok(())
}
