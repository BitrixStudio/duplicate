use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::ui::pane::PaneState;

pub fn handle_key(key: KeyEvent, panes: &mut [PaneState], active: &mut usize) -> Result<bool> {
    if key.kind != KeyEventKind::Press {
        return Ok(false);
    }

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