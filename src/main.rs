use anyhow::{anyhow, Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::{
    collections::VecDeque,
    io::{self, BufRead, BufReader, Read},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const SEP: &str = ":::";
const MAX_LINES: usize = 10_000;

#[derive(Parser, Debug)]
#[command(
    name = "duplicate",
    version,
    about = "Run N copies of a command and show split-screen output.",
    trailing_var_arg = true,
    disable_help_subcommand = true
)]
struct Cli {
    /// Number of instances to run
    #[arg(short = 'n', long = "n", default_value_t = 2)]
    n: usize,

    /// Command to run (e.g. ls, curl, rustlens)
    cmd: String,

    /// Remaining args (common args + per-instance args)
    args: Vec<String>,
    
    /// Run command through system shell (Windows: cmd.exe /C, Unix: sh -lc)
    #[arg(long = "shell")]
    shell: bool,
}

#[derive(Debug, Clone)]
enum StreamKind {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone)]
struct OutputEvent {
    pane: usize,
    stream: StreamKind,
    line: String,
}

#[derive(Debug)]
struct PaneState {
    title: String,
    lines: VecDeque<Line<'static>>,
    scroll: u16,
    follow: bool,
    exited: bool,
    viewport_height: u16, // NEW: last render height inside the border
}

impl PaneState {
    fn new(title: String) -> Self {
        Self {
            title,
            lines: VecDeque::new(),
            scroll: 0,
            follow: true,
            exited: false,
            viewport_height: 0,
        }
    }

    fn bottom_scroll(&self) -> u16 {
        let total = self.lines.len() as i32;
        let view = self.viewport_height as i32;

        let bottom = (total - view).max(0);
        bottom.min(u16::MAX as i32) as u16
    }

    fn push_line(&mut self, stream: &StreamKind, line: String) {
        let prefix = match stream {
            StreamKind::Stdout => "",
            StreamKind::Stderr => "stderr: ",
        };

        let owned: Line<'static> = Line::from(vec![Span::raw(prefix.to_string()), Span::raw(line)]);
        self.lines.push_back(owned);

        while self.lines.len() > MAX_LINES {
            self.lines.pop_front();
        }

        if self.follow {
            self.scroll = self.bottom_scroll();
        }
    }

    fn scroll_up(&mut self, n: u16) {
        self.follow = false;
        self.scroll = self.scroll.saturating_sub(n);
    }

    fn scroll_down(&mut self, n: u16) {
        self.follow = false;
        self.scroll = self.scroll.saturating_add(n).min(self.bottom_scroll());
    }

    fn scroll_top(&mut self) {
        self.follow = false;
        self.scroll = 0;
    }

    fn scroll_bottom(&mut self) {
        self.follow = true;
        self.scroll = self.bottom_scroll();
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.n == 0 {
        return Err(anyhow!("--n must be >= 1"));
    }

    let (common_args, instance_groups) = split_args(cli.n, &cli.args)?;
    if instance_groups.len() != cli.n {
        return Err(anyhow!(
            "Expected {} instance group(s), got {}",
            cli.n,
            instance_groups.len()
        ));
    }

    // Spawn children + output reader threads
    let (tx, rx) = mpsc::channel::<OutputEvent>();
    let mut children: Vec<Child> = Vec::with_capacity(cli.n);

    for i in 0..cli.n {
        let mut full_args = Vec::new();
        full_args.extend(common_args.iter().cloned());
        full_args.extend(instance_groups[i].iter().cloned());

        let mut child = spawn_instance(&cli.cmd, &full_args, cli.shell)
            .with_context(|| format!("Failed to spawn '{}'", cli.cmd))?;

        if let Some(stdout) = child.stdout.take() {
            spawn_reader(i, StreamKind::Stdout, stdout, tx.clone());
        }
        if let Some(stderr) = child.stderr.take() {
            spawn_reader(i, StreamKind::Stderr, stderr, tx.clone());
        }

        children.push(child);
    }

    // TUI
    enable_raw_mode().context("enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("enter alt screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("init terminal")?;

    let mut panes: Vec<PaneState> = (0..cli.n)
        .map(|i| {
            let title = format!(
                "#{i}  {} {}",
                cli.cmd,
                instance_groups[i].join(" ")
            );
            PaneState::new(title)
        })
        .collect();

    let mut active: usize = 0;
    let tick_rate = Duration::from_millis(33);
    let mut last_tick = Instant::now();

    let res = run_ui_loop(&mut terminal, &rx, &mut panes, &mut children, &mut active, tick_rate, &mut last_tick);

    // Cleanup terminal regardless of errors
    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    // Ensure children are terminated
    for mut c in children {
        let _ = c.kill();
        let _ = c.wait();
    }

    res
}

fn run_ui_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    rx: &mpsc::Receiver<OutputEvent>,
    panes: &mut [PaneState],
    children: &mut [Child],
    active: &mut usize,
    tick_rate: Duration,
    last_tick: &mut Instant,
) -> Result<()> {
    loop {
        // Drain output events quickly each frame
        while let Ok(ev) = rx.try_recv() {
            if let Some(p) = panes.get_mut(ev.pane) {
                p.push_line(&ev.stream, ev.line);
            }
        }

        // Mark panes as exited if process ended
        for (i, child) in children.iter_mut().enumerate() {
            if panes[i].exited {
                continue;
            }
            if let Ok(Some(status)) = child.try_wait() {
                panes[i].exited = true;
            
                if !status.success() {
                    panes[i].push_line(
                        &StreamKind::Stderr,
                        format!("[process exited: {status}]"),
                    );
                }
            }
        }

        terminal.draw(|f| {
            let size = f.size();
            let rects = grid_layout(size, panes.len());

            for (i, rect) in rects.into_iter().enumerate() {
                let is_active = i == *active;
                let mut title = panes[i].title.clone();
                if panes[i].follow {
                    title.push_str("  (follow)");
                }
                if panes[i].exited {
                    title.push_str("  (done)");
                }

                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .style(if is_active {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default()
                    });
                
                let inner_height = rect.height.saturating_sub(2);
                panes[i].viewport_height = inner_height;
                if panes[i].follow {
                    panes[i].scroll = panes[i].bottom_scroll();
                }

                let text = Text::from(panes[i].lines.iter().cloned().collect::<Vec<_>>());
                let paragraph = Paragraph::new(text)
                    .block(block)
                    .wrap(Wrap { trim: false })
                    .scroll((panes[i].scroll, 0));

                f.render_widget(paragraph, rect);
            }
        })?;

        // Exit when all are done and user not interacting (optional)
        // We'll keep running until user quits, because streaming commands may not exit
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(k) => {
                    // Windows terminals may emit Press + Repeat/Release; only act on Press
                    if k.kind != KeyEventKind::Press {
                        // ignore KeyEventKind::Release / KeyEventKind::Repeat
                    } else if handle_key(k, panes, active)? {
                        return Ok(());
                    }
                }
                Event::Resize(_, _) => {
                    // TODO: create resize in some future version
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            *last_tick = Instant::now();
        }
    }
}

fn handle_key(key: KeyEvent, panes: &mut [PaneState], active: &mut usize) -> Result<bool> {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => return Ok(true),

        (KeyCode::Tab, _) | (KeyCode::Char('i'), KeyModifiers::CONTROL) => {
            *active = (*active + 1) % panes.len();
        }

        (KeyCode::BackTab, _) => {
            *active = if *active == 0 { panes.len() - 1 } else { *active - 1 };
        }

        (KeyCode::Up, _) => panes[*active].scroll_up(1),
        (KeyCode::Down, _) => panes[*active].scroll_down(1),
        (KeyCode::PageUp, _) => panes[*active].scroll_up(10),
        (KeyCode::PageDown, _) => panes[*active].scroll_down(10),

        (KeyCode::Char('g'), KeyModifiers::NONE) => panes[*active].scroll_top(),
        (KeyCode::Char('G'), KeyModifiers::SHIFT) => panes[*active].scroll_bottom(),

        (KeyCode::Char('f'), _) => {
            let p = &mut panes[*active];
            p.follow = !p.follow;
            if p.follow {
                p.scroll_bottom();
            }
        }

        _ => {}
    }

    Ok(false)
}

fn grid_layout(area: Rect, n: usize) -> Vec<Rect> {
    if n == 0 {
        return vec![];
    }
    let (rows, cols) = if n <= 3 {
        (1, n)
    } else {
        let cols = (n as f64).sqrt().ceil() as usize;
        let rows = (n + cols - 1) / cols;
        (rows, cols)
    };

    let row_constraints = vec![Constraint::Ratio(1, rows as u32); rows];
    let rows_rects = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    let mut out = Vec::with_capacity(n);
    let mut idx = 0;

    for r in 0..rows {
        let col_constraints = vec![Constraint::Ratio(1, cols as u32); cols];
        let cols_rects = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints)
            .split(rows_rects[r]);

        for c in 0..cols {
            if idx >= n {
                break;
            }
            out.push(cols_rects[c]);
            idx += 1;
        }
    }

    out
}

fn spawn_reader<R: Read + Send + 'static>(pane: usize, stream: StreamKind, reader: R, tx: mpsc::Sender<OutputEvent>) {
    thread::spawn(move || {
        let mut br = BufReader::new(reader);
        let mut buf = Vec::new();

        loop {
            buf.clear();
            match br.read_until(b'\n', &mut buf) {
                Ok(0) => break,
                Ok(_) => {
                    let line = String::from_utf8_lossy(&buf).trim_end_matches(&['\n', '\r'][..]).to_string();
                    let _ = tx.send(OutputEvent { pane, stream: stream.clone(), line });
                }
                Err(_) => break,
            }
        }
    });
}

/// Split args into (common_args, instance_groups)
///
/// Rules:
/// 1) If `:::` appears, args before first `:::` are common_args
///    After that, we parse groups separated by `:::`
/// 2) If `:::` does NOT appear, we use shorthand:
///    - last N args become N groups (each group is one arg)
///    - everything before that is common_args
fn split_args(n: usize, args: &[String]) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    if let Some(first_sep) = args.iter().position(|a| a == SEP) {
        let common = args[..first_sep].to_vec();

        let mut groups: Vec<Vec<String>> = Vec::new();
        let mut current: Vec<String> = Vec::new();

        for tok in &args[first_sep + 1..] {
            if tok == SEP {
                groups.push(current);
                current = Vec::new();
            } else {
                current.push(tok.clone());
            }
        }
        groups.push(current);

        if groups.len() != n {
            return Err(anyhow!(
                "Got {} group(s) after '{SEP}', but --n is {n}. \
                 Tip: provide exactly {n} groups separated by '{SEP}'.",
                groups.len()
            ));
        }

        Ok((common, groups))
    } else {
        if args.len() < n {
            return Err(anyhow!(
                "Not enough args for shorthand mode: need at least {n} instance arg(s). \
                 Either add args or use '{SEP}' grouping."
            ));
        }
        let split_at = args.len() - n;
        let common = args[..split_at].to_vec();
        let groups = args[split_at..]
            .iter()
            .cloned()
            .map(|a| vec![a])
            .collect();
        Ok((common, groups))
    }
}

fn spawn_instance(cmd: &str, args: &[String], shell: bool) -> Result<std::process::Child> {
    // Try direct exec first unless --shell was requested
    if !shell {
        if let Ok(child) = Command::new(cmd)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            return Ok(child);
        }
        // If direct exec fails on Windows, fall back to shell
        #[cfg(windows)]
        {
            return spawn_via_shell(cmd, args);
        }
        #[cfg(not(windows))]
        {
            return Err(anyhow!("Failed to spawn '{cmd}'"));
        }
    }

    spawn_via_shell(cmd, args)
}

fn spawn_via_shell(cmd: &str, args: &[String]) -> Result<std::process::Child> {
    #[cfg(windows)]
    {
        // Build one command line for cmd.exe /C ...
        // NOTE: This is a pragmatic MVP; it won't perfectly preserve quoting for every edge case
        let mut line = String::new();
        line.push_str(cmd);
        for a in args {
            line.push(' ');
            line.push_str(&quote_cmd_arg_windows(a));
        }

        return Command::new("cmd")
            .args(["/C", &line])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn via cmd.exe");
    }

    #[cfg(not(windows))]
    {
        let mut line = String::new();
        line.push_str(cmd);
        for a in args {
            line.push(' ');
            line.push_str(&shell_escape_sh(a));
        }

        return Command::new("sh")
            .args(["-lc", &line])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn via sh");
    }
}

#[cfg(windows)]
fn quote_cmd_arg_windows(s: &str) -> String {
    // Simple quoting for cmd.exe: wrap in double quotes if it has spaces/specials
    // and escape embedded quotes
    let needs = s.chars().any(|c| c.is_whitespace() || r#""&|<>^"#.contains(c));
    if !needs {
        return s.to_string();
    }
    let escaped = s.replace('"', r#"\""#);
    format!(r#""{escaped}""#)
}

#[cfg(not(windows))]
fn shell_escape_sh(s: &str) -> String {
    // single-quote escape: ' -> '\'' 
    if s.is_empty() {
        return "''".to_string();
    }
    if !s.chars().any(|c| c.is_whitespace() || "'\"\\$`".contains(c)) {
        return s.to_string();
    }
    let mut out = String::from("'");
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}