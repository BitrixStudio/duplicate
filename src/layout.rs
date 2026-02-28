use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn grid_layout(area: Rect, n: usize) -> Vec<Rect> {
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