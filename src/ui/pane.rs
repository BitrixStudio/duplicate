use ratatui::text::{Line, Span};
use std::collections::VecDeque;

use crate::process::StreamKind;

const MAX_LINES: usize = 10_000;

#[derive(Debug)]
pub struct PaneState {
    pub title: String,
    pub lines: VecDeque<Line<'static>>,
    pub scroll: u16,
    pub follow: bool,
    pub exited: bool,
    pub viewport_height: u16,
}

impl PaneState {
    pub fn new(title: String) -> Self {
        Self {
            title,
            lines: VecDeque::new(),
            scroll: 0,
            follow: true,
            exited: false,
            viewport_height: 0,
        }
    }

    pub fn bottom_scroll(&self) -> u16 {
        let total = self.lines.len() as i32;
        let view = self.viewport_height as i32;
        let bottom = (total - view).max(0);
        bottom.min(u16::MAX as i32) as u16
    }

    pub fn push_line(&mut self, stream: &StreamKind, line: String) {
        let prefix = match stream {
            StreamKind::Stdout => "",
            StreamKind::Stderr => "stderr: ",
        };

        let owned: Line<'static> = Line::from(vec![
            Span::raw(prefix.to_string()),
            Span::raw(line),
        ]);

        self.lines.push_back(owned);
        while self.lines.len() > MAX_LINES {
            self.lines.pop_front();
        }

        if self.follow {
            self.scroll = self.bottom_scroll();
        }
    }

    pub fn scroll_up(&mut self, n: u16) {
        self.follow = false;
        self.scroll = self.scroll.saturating_sub(n);
    }

    pub fn scroll_down(&mut self, n: u16) {
        self.follow = false;
        self.scroll = self.scroll.saturating_add(n).min(self.bottom_scroll());
    }

    pub fn scroll_top(&mut self) {
        self.follow = false;
        self.scroll = 0;
    }

    pub fn scroll_bottom(&mut self) {
        self.follow = true;
        self.scroll = self.bottom_scroll();
    }
}