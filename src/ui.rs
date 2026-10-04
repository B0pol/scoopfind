use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Focus};

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search Input
            Constraint::Min(5),    // Main Results & Details Split
            Constraint::Length(1), // Footer / Status Bar
        ])
        .split(frame.area());

    draw_search_bar(frame, app, chunks[0]);
    draw_main_content(frame, app, chunks[1]);
    draw_footer(frame, app, chunks[2]);
}

fn draw_search_bar(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::SearchInput;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let search_title = if app.is_loading {
        " 🔍 Search Scoop Packages (Searching...) "
    } else {
        " 🔍 Search Scoop Packages "
    };

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            search_title,
            Style::default()
                .fg(if is_focused { Color::Cyan } else { Color::White })
                .add_modifier(Modifier::BOLD),
        ));

    let display_text = if app.query.is_empty() && !is_focused {
        Span::styled(
            "Type a package name (e.g. ripgrep, neovim, git)...",
            Style::default().fg(Color::DarkGray),
        )
    } else {
        Span::styled(&app.query, Style::default().fg(Color::White))
    };

    let paragraph = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        display_text,
    ]))
    .block(input_block);

    frame.render_widget(paragraph, area);

    // Show blinking/static terminal cursor in search box when focused
    if is_focused {
        frame.set_cursor_position((
            area.x + 2 + (app.cursor_position as u16) + 2, // "> " prefix is 2 chars + margin
            area.y + 1,
        ));
    }
}

fn draw_main_content(frame: &mut Frame, app: &App, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45), // Results list
            Constraint::Percentage(55), // Package Details
        ])
        .split(area);

    draw_results_list(frame, app, main_chunks[0]);
    draw_details_panel(frame, app, main_chunks[1]);
}

fn draw_results_list(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::ResultsList;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let title = format!(" Search Results ({}) ", app.results.len());
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            title,
            Style::default()
                .fg(if is_focused { Color::Cyan } else { Color::White })
                .add_modifier(Modifier::BOLD),
        ));

    if app.results.is_empty() {
        let empty_msg = if app.is_loading {
            "Searching Azure Search index..."
        } else if app.query.is_empty() {
            "Enter a query above and press <Enter>."
        } else {
            "No packages found."
        };

        let placeholder = Paragraph::new(empty_msg)
            .style(Style::default().fg(Color::DarkGray))
            .block(list_block);
        frame.render_widget(placeholder, area);
        return;
    }

    let items: Vec<ListItem> = app
        .results
        .iter()
        .enumerate()
        .map(|(idx, pkg)| {
            let is_selected = idx == app.selected_index;
            let bucket = pkg.bucket_name();

            let bucket_style = if pkg.metadata.official_repository {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Yellow)
            };

            let stars_display = match pkg.metadata.repository_stars {
                Some(s) if s > 0 => format!(" ★{}", s),
                _ => String::new(),
            };

            let prefix = if is_selected { "▶ " } else { "  " };

            let line = Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{:<16}", pkg.name),
                    Style::default()
                        .fg(if is_selected { Color::White } else { Color::Cyan })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("v{:<8}", pkg.version),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!("[{}]", bucket), bucket_style),
                Span::raw(" "),
                Span::styled(
                    format!("{:<4}", pkg.short_relative_age()),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(stars_display, Style::default().fg(Color::Yellow)),
            ]);

            let item_style = if is_selected {
                Style::default()
                    .bg(Color::Rgb(30, 45, 65))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(line).style(item_style)
        })
        .collect();

    let list = List::new(items).block(list_block);
    frame.render_widget(list, area);
}

fn draw_details_panel(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::DetailPane;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let details_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            " Package Information ",
            Style::default()
                .fg(if is_focused { Color::Cyan } else { Color::White })
                .add_modifier(Modifier::BOLD),
        ));

    let pkg = match app.selected_package() {
        Some(p) => p,
        None => {
            let placeholder = Paragraph::new("Select a package to view details and install commands.")
                .style(Style::default().fg(Color::DarkGray))
                .block(details_block);
            frame.render_widget(placeholder, area);
            return;
        }
    };

    let mut lines = Vec::new();

    // Package Header: Name and Version
    lines.push(Line::from(vec![
        Span::styled(&pkg.name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(format!("v{}", pkg.version), Style::default().fg(Color::Green)),
    ]));
    lines.push(Line::raw(""));

    // Description
    if let Some(desc) = &pkg.description {
        lines.push(Line::from(vec![
            Span::styled("Description: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(desc, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::raw(""));
    }

    // Bucket & Repository
    let bucket_type = if pkg.metadata.official_repository {
        "Official Bucket"
    } else {
        "Community Bucket"
    };

    lines.push(Line::from(vec![
        Span::styled("Bucket:      ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(pkg.bucket_name(), Style::default().fg(Color::White)),
        Span::styled(format!(" ({})", bucket_type), Style::default().fg(Color::DarkGray)),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Repository:  ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(&pkg.metadata.repository, Style::default().fg(Color::Blue)),
    ]));

    if let Some(homepage) = &pkg.homepage {
        lines.push(Line::from(vec![
            Span::styled("Homepage:    ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(homepage, Style::default().fg(Color::Blue)),
        ]));
    }

    if let Some(license) = &pkg.license {
        lines.push(Line::from(vec![
            Span::styled("License:     ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(license, Style::default().fg(Color::White)),
        ]));
    }

    // Updated Date & Relative Age
    let updated_info = format!("{} ({})", pkg.full_datetime_utc(), pkg.full_relative_age());
    lines.push(Line::from(vec![
        Span::styled("Updated:     ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(updated_info, Style::default().fg(Color::White)),
    ]));

    if let Some(sha) = &pkg.metadata.sha {
        lines.push(Line::from(vec![
            Span::styled("Commit SHA:  ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(sha, Style::default().fg(Color::DarkGray)),
        ]));
    }

    lines.push(Line::raw(""));

    // Install Command Box
    lines.push(Line::from(vec![
        Span::styled("Quick Install Command:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
    ]));

    let install_cmd = pkg.install_command();
    lines.push(Line::from(vec![
        Span::styled(
            format!("  > {}", install_cmd),
            Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(25, 30, 40))
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    // Notes if present
    if let Some(notes) = &pkg.notes {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("Notes:", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::styled(notes, Style::default().fg(Color::Gray)));
    }

    let paragraph = Paragraph::new(lines)
        .block(details_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let help_line = if area.width >= 90 {
        Line::from(vec![
            Span::styled("<Enter>", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" Search | "),
            Span::styled("<Escx2>", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" Clear | "),
            Span::styled("<Tab>", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" Focus | "),
            Span::styled("<Ctrl+Cx2>", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" / "),
            Span::styled("/quit", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" Exit "),
        ])
    } else {
        Line::from(vec![
            Span::styled("<Enter>", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" Search | "),
            Span::styled("<Escx2>", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" Clear | "),
            Span::styled("/quit", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" Exit "),
        ])
    };

    let help_width = (help_line.width() as u16).min(area.width);
    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(help_width),
        ])
        .split(area);

    let status_style = if app.is_loading {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else if app.key_refreshed {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let status_line = Line::from(vec![
        Span::styled(
            if app.is_loading { " ⏳ " } else { " ⚡ " },
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(&app.status_message, status_style),
    ]);

    let status_p = Paragraph::new(status_line);
    let help_p = Paragraph::new(help_line).alignment(ratatui::layout::Alignment::Right);

    frame.render_widget(status_p, footer_chunks[0]);
    frame.render_widget(help_p, footer_chunks[1]);
}
