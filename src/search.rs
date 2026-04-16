use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
};

/// Metadata about a variable used for filtering and display.
#[derive(Debug, Clone)]
pub struct VarInfo {
    pub name: String,
    pub group: String,
    pub dim_names: Vec<String>,
    pub dim_sizes: Vec<usize>,
    pub is_coord: bool,
}

impl VarInfo {
    /// Total number of elements in this variable.
    pub fn total_elements(&self) -> usize {
        self.dim_sizes.iter().product::<usize>().max(1)
    }

    /// Number of dimensions.
    pub fn ndim(&self) -> usize {
        self.dim_names.len()
    }
}

/// Dimension filter: match variables that use a specific dimension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DimFilter {
    /// Variable must use a dimension with this name.
    HasDim(String),
    /// Variable must have exactly this many dimensions.
    NDim(usize),
}

/// Search state for the variable tree.
pub struct SearchState {
    /// Current search query text (user-typed).
    pub query: String,
    /// Whether the search bar is active / focused.
    pub active: bool,
    /// Optional dimension filter applied alongside the text query.
    pub dim_filter: Option<DimFilter>,
    /// Full catalog of variables (populated once from the dataset).
    catalog: Vec<VarInfo>,
    /// Cached results of the last filter operation.
    results: Vec<usize>,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            active: false,
            dim_filter: None,
            catalog: Vec::new(),
            results: Vec::new(),
        }
    }

    /// Populate the variable catalog from a dataset.
    pub fn set_catalog(&mut self, vars: Vec<VarInfo>) {
        self.catalog = vars;
        self.refilter();
    }

    /// Push a character into the query.
    pub fn push_char(&mut self, ch: char) {
        self.query.push(ch);
        self.refilter();
    }

    /// Delete the last character from the query.
    pub fn pop_char(&mut self) {
        self.query.pop();
        self.refilter();
    }

    /// Clear the query and filter.
    pub fn clear(&mut self) {
        self.query.clear();
        self.dim_filter = None;
        self.refilter();
    }

    /// Set the dimension filter.
    pub fn set_dim_filter(&mut self, filter: Option<DimFilter>) {
        self.dim_filter = filter;
        self.refilter();
    }

    /// Returns indices into the catalog that match the current query + filter.
    pub fn matching_indices(&self) -> &[usize] {
        &self.results
    }

    /// Returns matching variable infos.
    pub fn matches(&self) -> Vec<&VarInfo> {
        self.results.iter().map(|&i| &self.catalog[i]).collect()
    }

    /// The full catalog.
    pub fn catalog(&self) -> &[VarInfo] {
        &self.catalog
    }

    /// Whether any filter is active.
    pub fn has_filter(&self) -> bool {
        !self.query.is_empty() || self.dim_filter.is_some()
    }

    fn refilter(&mut self) {
        let query_lower = self.query.to_lowercase();
        self.results = self
            .catalog
            .iter()
            .enumerate()
            .filter(|(_, v)| {
                // Text filter: fuzzy substring match on name, group, and dims
                let text_ok = if query_lower.is_empty() {
                    true
                } else {
                    fuzzy_match(&v.name.to_lowercase(), &query_lower)
                        || v.group.to_lowercase().contains(&query_lower)
                        || v.dim_names
                            .iter()
                            .any(|d| d.to_lowercase().contains(&query_lower))
                };

                // Dimension filter
                let dim_ok = match &self.dim_filter {
                    None => true,
                    Some(DimFilter::HasDim(name)) => v.dim_names.iter().any(|d| d == name),
                    Some(DimFilter::NDim(n)) => v.ndim() == *n,
                };

                text_ok && dim_ok
            })
            .map(|(i, _)| i)
            .collect();
    }

    /// Render the search bar into the given area.
    pub fn render_bar(&self, area: Rect, buf: &mut Buffer) {
        if area.height < 1 {
            return;
        }

        let border_color = if self.active {
            Color::Yellow
        } else {
            Color::Gray
        };

        let filter_tag = match &self.dim_filter {
            None => String::new(),
            Some(DimFilter::HasDim(d)) => format!(" [dim:{d}]"),
            Some(DimFilter::NDim(n)) => format!(" [{n}D]"),
        };

        let prompt = if self.active { "/" } else { "" };
        let status = if self.has_filter() {
            format!(" ({} match)", self.results.len())
        } else {
            String::new()
        };

        let line = Line::from(vec![
            Span::styled(
                format!(" {prompt}"),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                self.query.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(filter_tag, Style::default().fg(Color::Magenta)),
            Span::styled(status, Style::default().fg(Color::DarkGray)),
            if self.active {
                Span::styled("\u{2588}", Style::default().fg(Color::Yellow)) // cursor block
            } else {
                Span::raw("")
            },
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(" Search ");
        let inner = block.inner(area);
        Widget::render(block, area, buf);
        if inner.width > 0 && inner.height > 0 {
            Widget::render(line, inner, buf);
        }
    }

    /// Render a compact result list showing matched variables.
    pub fn render_results(&self, area: Rect, buf: &mut Buffer) {
        if !self.has_filter() || area.height < 2 {
            return;
        }
        let items: Vec<ListItem> = self
            .matches()
            .iter()
            .take(area.height as usize)
            .map(|v| {
                let dims_str: String = v
                    .dim_names
                    .iter()
                    .zip(&v.dim_sizes)
                    .map(|(n, s)| format!("{n}={s}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let style = if v.is_coord {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(&v.name, style),
                    Span::styled(format!("  [{dims_str}]"), Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .title(format!(" Results ({}) ", self.results.len()));

        Widget::render(List::new(items).block(block), area, buf);
    }
}

/// Simple fuzzy substring matcher.
///
/// Returns `true` if all characters in `pattern` appear in `text` in order
/// (not necessarily contiguous). This provides a typo-tolerant / fuzzy feel
/// for short variable names.
pub fn fuzzy_match(text: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }
    // Fast path: exact substring
    if text.contains(pattern) {
        return true;
    }
    // Subsequence match: all pattern chars appear in order
    let mut chars = text.chars();
    for pc in pattern.chars() {
        if !chars.any(|c| c == pc) {
            return false;
        }
    }
    true
}

/// Compute a simple fuzzy match score (higher = better match).
/// Returns 0 for no match. Used for ranking results.
pub fn fuzzy_score(text: &str, pattern: &str) -> u32 {
    if pattern.is_empty() {
        return 1;
    }
    let text_lower = text.to_lowercase();
    let pat_lower = pattern.to_lowercase();

    // Exact match is best
    if text_lower == pat_lower {
        return 1000;
    }
    // Starts-with gets high priority
    if text_lower.starts_with(&pat_lower) {
        return 500;
    }
    // Contains substring
    if text_lower.contains(&pat_lower) {
        return 200;
    }
    // Subsequence match
    if fuzzy_match(&text_lower, &pat_lower) {
        return 50;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_catalog() -> Vec<VarInfo> {
        vec![
            VarInfo {
                name: "temperature".into(),
                group: "atmosphere".into(),
                dim_names: vec!["lat".into(), "lon".into(), "time".into()],
                dim_sizes: vec![180, 360, 12],
                is_coord: false,
            },
            VarInfo {
                name: "pressure".into(),
                group: "atmosphere".into(),
                dim_names: vec!["lat".into(), "lon".into()],
                dim_sizes: vec![180, 360],
                is_coord: false,
            },
            VarInfo {
                name: "lat".into(),
                group: "atmosphere".into(),
                dim_names: vec!["lat".into()],
                dim_sizes: vec![180],
                is_coord: true,
            },
            VarInfo {
                name: "salinity".into(),
                group: "ocean".into(),
                dim_names: vec!["depth".into()],
                dim_sizes: vec![50],
                is_coord: false,
            },
        ]
    }

    #[test]
    fn test_empty_query_matches_all() {
        let mut ss = SearchState::new();
        ss.set_catalog(sample_catalog());
        assert_eq!(ss.matching_indices().len(), 4);
    }

    #[test]
    fn test_substring_filter() {
        let mut ss = SearchState::new();
        ss.set_catalog(sample_catalog());
        ss.push_char('t');
        ss.push_char('e');
        ss.push_char('m');
        ss.push_char('p');
        // "temp" matches "temperature"
        assert!(ss.matches().iter().any(|v| v.name == "temperature"));
    }

    #[test]
    fn test_fuzzy_filter() {
        let mut ss = SearchState::new();
        ss.set_catalog(sample_catalog());
        // "tpr" should fuzzy-match "temperature" (t-e-m-p-e-r... t, p, r in order)
        for ch in "tpr".chars() {
            ss.push_char(ch);
        }
        let names: Vec<&str> = ss.matches().iter().map(|v| v.name.as_str()).collect();
        assert!(names.contains(&"temperature"));
    }

    #[test]
    fn test_dim_filter_has_dim() {
        let mut ss = SearchState::new();
        ss.set_catalog(sample_catalog());
        ss.set_dim_filter(Some(DimFilter::HasDim("depth".into())));
        assert_eq!(ss.matches().len(), 1);
        assert_eq!(ss.matches()[0].name, "salinity");
    }

    #[test]
    fn test_dim_filter_ndim() {
        let mut ss = SearchState::new();
        ss.set_catalog(sample_catalog());
        ss.set_dim_filter(Some(DimFilter::NDim(2)));
        let names: Vec<&str> = ss.matches().iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, vec!["pressure"]);
    }

    #[test]
    fn test_combined_text_and_dim_filter() {
        let mut ss = SearchState::new();
        ss.set_catalog(sample_catalog());
        for ch in "lat".chars() {
            ss.push_char(ch);
        }
        // "lat" matches: temperature (has dim lat), pressure (has dim lat), lat (name+dim)
        // Now add dim filter for 1D:
        ss.set_dim_filter(Some(DimFilter::NDim(1)));
        let names: Vec<&str> = ss.matches().iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, vec!["lat"]);
    }

    #[test]
    fn test_clear_resets_everything() {
        let mut ss = SearchState::new();
        ss.set_catalog(sample_catalog());
        ss.push_char('x');
        ss.set_dim_filter(Some(DimFilter::NDim(5)));
        assert_eq!(ss.matches().len(), 0);
        ss.clear();
        assert_eq!(ss.matches().len(), 4);
        assert!(!ss.has_filter());
    }

    #[test]
    fn test_pop_char() {
        let mut ss = SearchState::new();
        ss.set_catalog(sample_catalog());
        ss.push_char('z');
        ss.push_char('z');
        assert_eq!(ss.matches().len(), 0);
        ss.pop_char();
        ss.pop_char();
        assert_eq!(ss.matches().len(), 4);
    }

    #[test]
    fn test_group_search() {
        let mut ss = SearchState::new();
        ss.set_catalog(sample_catalog());
        for ch in "ocean".chars() {
            ss.push_char(ch);
        }
        let names: Vec<&str> = ss.matches().iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, vec!["salinity"]);
    }

    #[test]
    fn test_dim_name_search() {
        let mut ss = SearchState::new();
        ss.set_catalog(sample_catalog());
        for ch in "depth".chars() {
            ss.push_char(ch);
        }
        let names: Vec<&str> = ss.matches().iter().map(|v| v.name.as_str()).collect();
        assert!(names.contains(&"salinity")); // has dim "depth"
    }

    #[test]
    fn test_fuzzy_match_fn() {
        assert!(fuzzy_match("temperature", "temp"));
        assert!(fuzzy_match("temperature", "tpr"));
        assert!(fuzzy_match("temperature", "tmprtr"));
        assert!(!fuzzy_match("temperature", "xyz"));
        assert!(fuzzy_match("anything", ""));
    }

    #[test]
    fn test_fuzzy_score() {
        assert_eq!(fuzzy_score("temp", "temp"), 1000); // exact
        assert!(fuzzy_score("temperature", "temp") > fuzzy_score("temperature", "tpr"));
        assert_eq!(fuzzy_score("abc", "xyz"), 0); // no match
    }

    #[test]
    fn test_var_info_helpers() {
        let v = VarInfo {
            name: "temp".into(),
            group: "/".into(),
            dim_names: vec!["lat".into(), "lon".into()],
            dim_sizes: vec![180, 360],
            is_coord: false,
        };
        assert_eq!(v.ndim(), 2);
        assert_eq!(v.total_elements(), 64800);
    }
}
