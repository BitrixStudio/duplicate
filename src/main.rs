use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    process::Child,
    sync::mpsc,
    time::{Duration, Instant},
};

mod args;
mod cli;
mod layout;
mod process;
mod ui;

use args::split_args;
use cli::Cli;
use process::{spawn_children, OutputEvent};
use ui::{app::App, input::handle_key, pane::PaneState};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (common_args, instance_groups) = split_args(cli.n, &cli.args)?;

    let (tx, rx) = mpsc::channel::<OutputEvent>();
    let mut children = spawn_children(cli.n, &cli.cmd, &common_args, &instance_groups, tx)?;

    let panes = (0..cli.n)
        .map(|i| {
            let title = format!("#{}  {} {}", i, cli.cmd, instance_groups[i].join(" "));
            PaneState::new(title)
        })
        .collect();

    let mut app = App::new(panes);

    enable_raw_mode().context("enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("enter alt screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("init terminal")?;

    let res = run_ui_loop(&mut terminal, &rx, &mut app, &mut children);

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    for mut c in children {
        let _ = c.kill();
        let _ = c.wait();
    }

    res
}

fn run_ui_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    rx: &mpsc::Receiver<OutputEvent>,
    app: &mut App,
    children: &mut [Child],
) -> Result<()> {
    let tick_rate = Duration::from_millis(33);
    let mut last_tick = Instant::now();

    loop {
        while let Ok(ev) = rx.try_recv() {
            if let Some(p) = app.panes.get_mut(ev.pane) {
                p.push_line(&ev.stream, ev.line);
            }
        }

        for (i, child) in children.iter_mut().enumerate() {
            if app.panes[i].exited {
                continue;
            }
            if let Ok(Some(status)) = child.try_wait() {
                app.panes[i].exited = true;
                if !status.success() {
                    app.panes[i].push_line(&process::StreamKind::Stderr, format!("[process exited: {status}]"));
                }
            }
        }

        terminal.draw(|f| app.draw(f))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));

        if event::poll(timeout)? {
            if let event::Event::Key(k) = event::read()? {
                if handle_key(k, &mut app.panes, &mut app.active)? {
                    return Ok(());
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}