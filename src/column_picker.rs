use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, StatefulWidget, Widget},
};
use std::collections::HashSet;

pub struct ColumnPickerState {
    headers: Vec<String>,
    selected: HashSet<usize>,
    cursor: usize,
}

impl ColumnPickerState {
    pub fn new(headers: &[String], selected: impl IntoIterator<Item = usize>) -> Self {
        Self {
            headers: headers.to_vec(),
            selected: selected.into_iter().collect(),
            cursor: 0,
        }
    }

    pub fn matching_indices(&self, query: &str) -> Vec<usize> {
        let mut matches: Vec<(usize, usize)> = self
            .headers
            .iter()
            .enumerate()
            .filter_map(|(index, header)| fuzzy_score(header, query).map(|score| (index, score)))
            .collect();
        matches.sort_by_key(|(index, score)| (*score, *index));
        matches.into_iter().map(|(index, _)| index).collect()
    }

    pub fn move_cursor(&mut self, query: &str, delta: isize) {
        let len = self.matching_indices(query).len();
        if len == 0 {
            self.cursor = 0;
        } else {
            self.cursor = (self.cursor as isize + delta).rem_euclid(len as isize) as usize;
        }
    }

    pub fn toggle_current(&mut self, query: &str) {
        let matches = self.matching_indices(query);
        if let Some(&index) = matches.get(self.cursor.min(matches.len().saturating_sub(1))) {
            if !self.selected.remove(&index) {
                self.selected.insert(index);
            }
        }
    }

    pub fn select_matches(&mut self, query: &str, selected: bool) {
        for index in self.matching_indices(query) {
            if selected {
                self.selected.insert(index);
            } else {
                self.selected.remove(&index);
            }
        }
    }

    pub fn reset_cursor(&mut self, query: &str) {
        let len = self.matching_indices(query).len();
        self.cursor = self.cursor.min(len.saturating_sub(1));
    }

    pub fn selected_indices(&self) -> Vec<usize> {
        let mut indices: Vec<_> = self.selected.iter().copied().collect();
        indices.sort_unstable();
        indices
    }

    fn is_selected(&self, index: usize) -> bool {
        self.selected.contains(&index)
    }
}

/// Case-insensitive subsequence match. Lower scores indicate a tighter match.
fn fuzzy_score(candidate: &str, query: &str) -> Option<usize> {
    let query = query.to_lowercase();
    if query.is_empty() {
        return Some(0);
    }
    let mut score = 0;
    let mut previous = None;
    let mut chars = query.chars();
    let mut wanted = chars.next()?;
    for (position, character) in candidate.to_lowercase().chars().enumerate() {
        if character == wanted {
            if let Some(previous) = previous {
                score += position.saturating_sub(previous + 1);
            } else {
                score += position;
            }
            previous = Some(position);
            if let Some(next) = chars.next() {
                wanted = next;
            } else {
                return Some(score);
            }
        }
    }
    None
}

pub struct ColumnPicker<'a> {
    query: &'a str,
}

impl<'a> ColumnPicker<'a> {
    pub fn new(query: &'a str) -> Self {
        Self { query }
    }
}

impl StatefulWidget for ColumnPicker<'_> {
    type State = ColumnPickerState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let width = area.width.min(70).max(1);
        let height = area.height.min(20).max(1);
        let popup = Rect::new(
            area.x + area.width.saturating_sub(width) / 2,
            area.y + area.height.saturating_sub(height) / 2,
            width,
            height,
        );
        Clear.render(popup, buf);
        let matches = state.matching_indices(self.query);
        state.reset_cursor(self.query);
        let available = popup.height.saturating_sub(4) as usize;
        let start = state.cursor.saturating_sub(available.saturating_sub(1));
        let mut lines = vec![Line::from(format!(
            "Search: {}  ({}/{} selected)",
            self.query,
            state.selected.len(),
            state.headers.len()
        ))];
        for (position, index) in matches.iter().skip(start).take(available).enumerate() {
            let checked = if state.is_selected(*index) { "x" } else { " " };
            let marker = if start + position == state.cursor {
                ">"
            } else {
                " "
            };
            let style = if start + position == state.cursor {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(
                format!(
                    "{marker} [{checked}] {}: {}",
                    index + 1,
                    state.headers[*index]
                ),
                style,
            )));
        }
        if matches.is_empty() {
            lines.push(Line::from("  No matching columns"));
        }
        lines.push(Line::from(
            "Tab toggle • Enter apply • Esc cancel • Ctrl+A/Ctrl+D all/none",
        ));
        let block = Block::default()
            .title(" Select columns ")
            .borders(Borders::ALL);
        ratatui::widgets::Paragraph::new(lines)
            .block(block)
            .render(popup, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_matching_and_selection_are_independent() {
        let mut picker = ColumnPickerState::new(&["first_name".into(), "last_name".into()], [0]);
        assert_eq!(picker.matching_indices("fn"), vec![0]);
        picker.toggle_current("ln");
        assert_eq!(picker.selected_indices(), vec![0, 1]);
    }
}
