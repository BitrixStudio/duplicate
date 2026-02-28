use ratatui::{
    style::{Modifier, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{layout::grid_layout, ui::pane::PaneState};

pub struct App {
    pub panes: Vec<PaneState>,
    pub active: usize,
}

impl App {
    pub fn new(panes: Vec<PaneState>) -> Self {
        Self { panes, active: 0 }
    }

    pub fn draw(&mut self, f: &mut Frame) {
        let size = f.size();
        let rects = grid_layout(size, self.panes.len());

        for (i, rect) in rects.into_iter().enumerate() {
            let is_active = i == self.active;

            let mut title = self.panes[i].title.clone();
            if self.panes[i].follow {
                title.push_str("  (follow)");
            }
            if self.panes[i].exited {
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
            self.panes[i].viewport_height = inner_height;
            if self.panes[i].follow {
                self.panes[i].scroll = self.panes[i].bottom_scroll();
            }

            let text = Text::from(self.panes[i].lines.iter().cloned().collect::<Vec<_>>());
            let paragraph = Paragraph::new(text)
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((self.panes[i].scroll, 0));

            f.render_widget(paragraph, rect);
        }
    }
}