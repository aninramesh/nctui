#[cfg(not(feature = "netcdf-backend"))]
fn main() {
    eprintln!(
        "nctui v{} — built without NetCDF backend.\n\
         Rebuild with: cargo build --features netcdf-backend",
        env!("CARGO_PKG_VERSION")
    );
    std::process::exit(1);
}

#[cfg(feature = "netcdf-backend")]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 || args[1] == "-h" || args[1] == "--help" {
        eprintln!(
            "nctui v{} — Terminal UI viewer for NetCDF4 / HDF5 datasets",
            env!("CARGO_PKG_VERSION")
        );
        eprintln!("Usage: nctui <file.nc>");
        std::process::exit(if args.len() == 1 { 1 } else { 0 });
    }

    let path = std::path::Path::new(&args[1]);
    let (file, info) = match nctui::backend::open_dataset(path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = run_tui(path, file, info) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

#[cfg(feature = "netcdf-backend")]
fn run_tui(
    path: &std::path::Path,
    file: netcdf::File,
    info: nctui::backend::DatasetInfo,
) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::{
        event::{self, Event},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use nctui::app::App;
    use ratatui::{backend::CrosstermBackend, Terminal};
    use std::io;

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(path.display().to_string(), &info);

    // Auto-expand the first group on startup for discoverability
    if !app.tree.rows.is_empty() {
        app.tree.toggle_expand();
    }

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();
            nctui::ui::draw(&app, area, buf);
        })?;

        if !app.running {
            break;
        }

        if let Event::Key(key) = event::read()? {
            handle_key(&mut app, key, &file, &info);
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    Ok(())
}

#[cfg(feature = "netcdf-backend")]
fn handle_key(
    app: &mut nctui::app::App,
    key: crossterm::event::KeyEvent,
    file: &netcdf::File,
    info: &nctui::backend::DatasetInfo,
) {
    use crossterm::event::{KeyCode, KeyModifiers};
    use nctui::app::{Focus, Modal};
    use nctui::slice_picker::DimRole;

    // Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.running = false;
        return;
    }

    // Modal-specific handling takes priority
    match app.modal {
        Modal::Help => {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                    app.modal = Modal::None;
                }
                _ => {}
            }
            return;
        }

        Modal::Histogram => {
            match key.code {
                KeyCode::Esc | KeyCode::Char('h') => {
                    app.histogram.visible = false;
                    app.modal = Modal::None;
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    app.histogram.increase_bins();
                    if !app.current_data.is_empty() {
                        app.histogram.set_data(&app.current_data);
                    }
                }
                KeyCode::Char('-') => {
                    app.histogram.decrease_bins();
                    if !app.current_data.is_empty() {
                        app.histogram.set_data(&app.current_data);
                    }
                }
                _ => {}
            }
            return;
        }

        Modal::Table => {
            if let Some(ref mut table) = app.table {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('t') => {
                        table.visible = false;
                        app.modal = Modal::None;
                    }
                    KeyCode::Down | KeyCode::Char('j') => table.scroll_down(1),
                    KeyCode::Up | KeyCode::Char('k') => table.scroll_up(1),
                    KeyCode::Right | KeyCode::Char('l') => table.scroll_right(1),
                    KeyCode::Left | KeyCode::Char('h') => table.scroll_left(1),
                    KeyCode::PageDown => table.scroll_down(20),
                    KeyCode::PageUp => table.scroll_up(20),
                    _ => {}
                }
            }
            return;
        }

        Modal::SlicePicker => {
            if let Some(ref mut picker) = app.slice_picker {
                match key.code {
                    KeyCode::Esc => {
                        picker.visible = false;
                        app.slice_picker = None;
                        app.modal = Modal::None;
                        app.status_msg.clear();
                    }
                    KeyCode::Enter => {
                        if picker.spec.free_dim_count() == 2 {
                            app.apply_slice(file, info);
                        } else {
                            app.status_msg = "Assign exactly 2 free axes (X and Y)".to_string();
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if picker.selected + 1 < picker.spec.dims.len() {
                            picker.selected += 1;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if picker.selected > 0 {
                            picker.selected -= 1;
                        }
                    }
                    KeyCode::Char('x') => {
                        picker.spec.assign_axis(picker.selected, DimRole::AxisX);
                    }
                    KeyCode::Char('y') => {
                        picker.spec.assign_axis(picker.selected, DimRole::AxisY);
                    }
                    KeyCode::Char('f') => {
                        picker.spec.assign_axis(picker.selected, DimRole::Fixed(0));
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        picker.spec.increment_fixed(picker.selected);
                    }
                    KeyCode::Char('h') | KeyCode::Left => {
                        picker.spec.decrement_fixed(picker.selected);
                    }
                    _ => {}
                }
            }
            return;
        }

        Modal::None => {}
    }

    // Search mode
    if app.focus == Focus::Search {
        match key.code {
            KeyCode::Esc => {
                app.search.active = false;
                app.search.clear();
                app.focus = Focus::Tree;
            }
            KeyCode::Enter => {
                // If search has results, select the first match and load it
                app.search.active = false;
                app.focus = Focus::Tree;
                // Keep the filter visible but return focus to tree
            }
            KeyCode::Backspace => {
                app.search.pop_char();
            }
            KeyCode::Char(ch) => {
                app.search.push_char(ch);
            }
            _ => {}
        }
        return;
    }

    // Normal tree navigation mode
    match key.code {
        KeyCode::Char('q') => {
            app.running = false;
        }
        KeyCode::Char('?') => {
            app.modal = Modal::Help;
        }
        KeyCode::Char('/') => {
            app.focus = Focus::Search;
            app.search.active = true;
        }
        KeyCode::Esc => {
            // Clear search filter if active
            if app.search.has_filter() {
                app.search.clear();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.tree.move_down();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.tree.move_up();
        }
        KeyCode::Char('g') => {
            app.tree.jump_top();
        }
        KeyCode::Char('G') => {
            app.tree.jump_bottom();
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            app.load_selected_variable(file, info);
        }
        KeyCode::Char('h') => {
            if app.current_var.is_some() {
                app.histogram.visible = true;
                app.modal = Modal::Histogram;
            }
        }
        KeyCode::Char('t') => {
            if let Some(ref mut table) = app.table {
                table.visible = true;
                app.modal = Modal::Table;
            }
        }
        KeyCode::Char('s') => {
            // Open slice picker for current variable if nD
            if let Some(row) = app.tree.rows.get(app.tree.selected) {
                if let Some(meta) = app.var_meta.get(&row.label) {
                    if meta.dim_names.len() > 2 {
                        let spec = nctui::slice_picker::SliceSpec::default_for(
                            &row.label,
                            &meta.dim_names,
                            &meta.dim_sizes,
                        );
                        app.slice_picker = Some(nctui::slice_picker::SlicePicker::new(spec));
                        app.modal = Modal::SlicePicker;
                        app.status_msg = format!("{}: pick a 2D slice", row.label);
                    }
                }
            }
        }
        _ => {}
    }
}
