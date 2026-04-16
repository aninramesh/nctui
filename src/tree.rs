use indexmap::IndexMap;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget},
};

/// Kind of row in the tree navigator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowKind {
    Group { expanded: bool },
    Variable { is_coord: bool },
}

/// A single visible row in the flattened tree.
#[derive(Debug, Clone)]
pub struct Row {
    pub label: String,
    pub path: String,
    pub kind: RowKind,
    pub indent: usize,
}

/// Left-panel tree navigator for variables and groups.
pub struct TreeNavigator {
    pub rows: Vec<Row>,
    pub selected: usize,
    pub expanded: std::collections::HashSet<String>,
    groups: IndexMap<String, Vec<String>>,
    variables: IndexMap<String, Vec<String>>,
}

impl TreeNavigator {
    /// Build a tree from group→children and variable→dims mappings.
    pub fn new(
        groups: IndexMap<String, Vec<String>>,
        variables: IndexMap<String, Vec<String>>,
    ) -> Self {
        let mut nav = Self {
            rows: Vec::new(),
            selected: 0,
            expanded: std::collections::HashSet::new(),
            groups,
            variables,
        };
        nav.rebuild_rows();
        nav
    }

    /// Rebuild the flat visible-row list from the expanded set.
    pub fn rebuild_rows(&mut self) {
        self.rows.clear();
        // Root groups first
        for (name, children) in &self.groups {
            let path = format!("/{name}");
            let expanded = self.expanded.contains(&path);
            self.rows.push(Row {
                label: name.clone(),
                path: path.clone(),
                kind: RowKind::Group { expanded },
                indent: 0,
            });
            if expanded {
                for child in children {
                    if let Some(dims) = self.variables.get(child) {
                        let is_coord = dims.len() == 1 && dims[0] == *child;
                        self.rows.push(Row {
                            label: child.clone(),
                            path: format!("/{name}/{child}"),
                            kind: RowKind::Variable { is_coord },
                            indent: 1,
                        });
                    }
                }
            }
        }
        // Clamp selection
        if !self.rows.is_empty() && self.selected >= self.rows.len() {
            self.selected = self.rows.len() - 1;
        }
    }

    /// Toggle expand/collapse on the selected row if it is a group.
    pub fn toggle_expand(&mut self) {
        if let Some(row) = self.rows.get(self.selected) {
            if matches!(row.kind, RowKind::Group { .. }) {
                let path = row.path.clone();
                if self.expanded.contains(&path) {
                    self.expanded.remove(&path);
                } else {
                    self.expanded.insert(path);
                }
                self.rebuild_rows();
            }
        }
    }

    /// Move selection down.
    pub fn move_down(&mut self) {
        if self.selected + 1 < self.rows.len() {
            self.selected += 1;
        }
    }

    /// Move selection up.
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Jump to first row.
    pub fn jump_top(&mut self) {
        self.selected = 0;
    }

    /// Jump to last row.
    pub fn jump_bottom(&mut self) {
        if !self.rows.is_empty() {
            self.selected = self.rows.len() - 1;
        }
    }

    /// Render the tree into a buffer area.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let indent = "  ".repeat(row.indent);
                let icon = match &row.kind {
                    RowKind::Group { expanded: true } => "\u{25bc} ", // ▼
                    RowKind::Group { expanded: false } => "\u{25b6} ", // ▶
                    RowKind::Variable { is_coord: true } => "\u{25c6} ", // ◆
                    RowKind::Variable { is_coord: false } => "  ",
                };
                let cursor = if i == self.selected { "\u{203a} " } else { "  " }; // ›
                let style = if i == self.selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    match &row.kind {
                        RowKind::Group { .. } => Style::default().fg(Color::Cyan),
                        RowKind::Variable { is_coord: true } => {
                            Style::default().fg(Color::DarkGray)
                        }
                        RowKind::Variable { is_coord: false } => Style::default().fg(Color::White),
                    }
                };
                let line = Line::from(vec![
                    Span::raw(cursor),
                    Span::raw(indent),
                    Span::styled(format!("{icon}{}", row.label), style),
                ]);
                ListItem::new(line)
            })
            .collect();

        let block = Block::default()
            .title(" Variables ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));

        let mut state = ListState::default();
        state.select(Some(self.selected));
        StatefulWidget::render(List::new(items).block(block), area, buf, &mut state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub_tree() -> TreeNavigator {
        let mut groups = IndexMap::new();
        groups.insert(
            "atmosphere".to_string(),
            vec![
                "temperature".to_string(),
                "pressure".to_string(),
                "lat".to_string(),
            ],
        );
        groups.insert("ocean".to_string(), vec!["salinity".to_string()]);

        let mut vars = IndexMap::new();
        vars.insert(
            "temperature".to_string(),
            vec!["lat".to_string(), "lon".to_string(), "time".to_string()],
        );
        vars.insert(
            "pressure".to_string(),
            vec!["lat".to_string(), "lon".to_string()],
        );
        vars.insert("lat".to_string(), vec!["lat".to_string()]);
        vars.insert("salinity".to_string(), vec!["depth".to_string()]);

        TreeNavigator::new(groups, vars)
    }

    #[test]
    fn test_initial_rows_all_collapsed() {
        let tree = stub_tree();
        assert_eq!(tree.rows.len(), 2);
        assert!(matches!(
            tree.rows[0].kind,
            RowKind::Group { expanded: false }
        ));
        assert!(matches!(
            tree.rows[1].kind,
            RowKind::Group { expanded: false }
        ));
    }

    #[test]
    fn test_expand_group_rebuilds_rows() {
        let mut tree = stub_tree();
        tree.toggle_expand(); // expand "atmosphere"
        assert_eq!(tree.rows.len(), 5); // atmosphere + 3 children + ocean
        assert!(matches!(
            tree.rows[0].kind,
            RowKind::Group { expanded: true }
        ));
        assert!(matches!(
            tree.rows[1].kind,
            RowKind::Variable { is_coord: false }
        ));
    }

    #[test]
    fn test_collapse_restores_row_count() {
        let mut tree = stub_tree();
        tree.toggle_expand();
        assert_eq!(tree.rows.len(), 5);
        tree.selected = 0;
        tree.toggle_expand();
        assert_eq!(tree.rows.len(), 2);
    }

    #[test]
    fn test_coord_detection() {
        let mut tree = stub_tree();
        tree.toggle_expand(); // expand atmosphere
        // lat has dims=["lat"], so is_coord=true
        let lat_row = tree.rows.iter().find(|r| r.label == "lat").unwrap();
        assert!(matches!(
            lat_row.kind,
            RowKind::Variable { is_coord: true }
        ));
    }

    #[test]
    fn test_selection_clamp_on_collapse() {
        let mut tree = stub_tree();
        tree.toggle_expand(); // expand atmosphere: 5 rows
        tree.selected = 4; // last row (ocean)
        tree.selected = 0;
        tree.toggle_expand(); // collapse atmosphere: 2 rows
        assert!(tree.selected < tree.rows.len());
    }

    #[test]
    fn test_move_down_up() {
        let mut tree = stub_tree();
        assert_eq!(tree.selected, 0);
        tree.move_down();
        assert_eq!(tree.selected, 1);
        tree.move_down();
        assert_eq!(tree.selected, 1); // can't go past end
        tree.move_up();
        assert_eq!(tree.selected, 0);
        tree.move_up();
        assert_eq!(tree.selected, 0); // can't go past start
    }

    #[test]
    fn test_jump_top_bottom() {
        let mut tree = stub_tree();
        tree.toggle_expand();
        tree.jump_bottom();
        assert_eq!(tree.selected, tree.rows.len() - 1);
        tree.jump_top();
        assert_eq!(tree.selected, 0);
    }
}
