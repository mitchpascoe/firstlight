use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, ListState, Padding},
    DefaultTerminal, Frame,
};
use ratatui_image::StatefulImage;

use crate::esa::{self, EsaImage};
use crate::preview::PreviewManager;
use crate::wallpaper;

const HIGHLIGHT: Color = Color::Rgb(196, 167, 231); // #c4a7e7
const BORDER: Color = Color::Rgb(110, 106, 134); // #6e6a86
const TITLE_COLOR: Color = Color::Rgb(224, 222, 244); // #e0def4
const MUTED: Color = Color::Rgb(144, 140, 170); // #908caa

pub struct App {
    images: Vec<EsaImage>,
    selected: usize,
    preview: PreviewManager,
    status: Option<String>,
    filter: String,
    filtering: bool,
    should_exit: bool,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let images = esa::fetch_images()?;
        let preview = PreviewManager::new();

        Ok(Self {
            images,
            selected: 0,
            preview,
            status: None,
            filter: String::new(),
            filtering: false,
            should_exit: false,
        })
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        while !self.should_exit {
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key);
                }
            }
        }
        Ok(())
    }

    fn filtered_indices(&self) -> Vec<usize> {
        if self.filter.is_empty() {
            (0..self.images.len()).collect()
        } else {
            let q = self.filter.to_lowercase();
            self.images
                .iter()
                .enumerate()
                .filter_map(|(i, img)| img.title.to_lowercase().contains(&q).then_some(i))
                .collect()
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_exit = true;
            return;
        }

        if self.filtering {
            self.handle_filter_key(key);
            return;
        }

        let indices = self.filtered_indices();
        let len = indices.len();

        match key.code {
            KeyCode::Char('q') => self.should_exit = true,
            KeyCode::Char('j') | KeyCode::Down => {
                if len > 0 {
                    self.selected = (self.selected + 1).min(len - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.selected = 0;
            }
            KeyCode::End => {
                if len > 0 {
                    self.selected = len - 1;
                }
            }
            KeyCode::Char('/') => {
                self.filtering = true;
                self.filter.clear();
                self.status = None;
            }
            KeyCode::Enter => {
                if let Some(&idx) = indices.get(self.selected) {
                    let img = self.images[idx].clone();
                    self.set_wallpaper(img);
                }
            }
            _ => {}
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.filtering = false;
                self.filter.clear();
                self.selected = 0;
            }
            KeyCode::Enter => {
                self.filtering = false;
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.selected = 0;
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                self.selected = 0;
            }
            _ => {}
        }
    }

    fn set_wallpaper(&mut self, img: EsaImage) {
        self.status = Some(format!("Downloading {}...", img.title));

        match esa::download(&img.wallpaper_url, "wall", &img.id) {
            Ok(path) => match wallpaper::set_wallpaper(&path) {
                Ok(()) => self.status = Some(format!("Wallpaper set: {}", img.title)),
                Err(e) => self.status = Some(format!("Error: {e}")),
            },
            Err(e) => self.status = Some(format!("Download failed: {e}")),
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let indices = self.filtered_indices();
        self.selected = self.selected.min(indices.len().saturating_sub(1));

        let [main_area, help_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

        let [list_area, preview_area] =
            Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
                .areas(main_area);

        self.draw_list(frame, list_area, &indices);
        self.draw_preview(frame, preview_area, &indices);
        self.draw_help(frame, help_area);
    }

    fn draw_list(&mut self, frame: &mut Frame, area: Rect, indices: &[usize]) {
        let title = if self.filtering {
            format!(" Filter: {}_ ", self.filter)
        } else {
            " Firstlight ".to_string()
        };

        let items: Vec<ListItem> = indices
            .iter()
            .map(|&i| ListItem::new(self.images[i].title.as_str()))
            .collect();

        let list = List::new(items)
            .block(
                Block::bordered()
                    .border_style(Style::new().fg(BORDER))
                    .border_type(BorderType::Rounded)
                    .title(Span::from(title).fg(TITLE_COLOR).bold())
                    .padding(Padding::horizontal(1)),
            )
            .highlight_style(Style::new().fg(HIGHLIGHT).bold())
            .highlight_symbol("> ");

        let mut state = ListState::default().with_selected(Some(self.selected));
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn draw_preview(&mut self, frame: &mut Frame, area: Rect, indices: &[usize]) {
        let Some(&idx) = indices.get(self.selected) else {
            frame.render_widget(preview_block(""), area);
            return;
        };

        let img = &self.images[idx];
        let date_title = format_date(&img.date);
        let id = img.id.clone();
        let preview_url = img.preview_url.clone();

        let block = preview_block(&date_title);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        self.preview.ensure_loaded(&id, &preview_url);

        if let Some(protocol) = self.preview.get_mut(&id) {
            frame.render_stateful_widget(StatefulImage::default(), inner, protocol);
        }
    }

    fn draw_help(&self, frame: &mut Frame, area: Rect) {
        let text = if self.filtering {
            " esc cancel  enter apply  type to filter".to_string()
        } else if let Some(status) = &self.status {
            format!(" {status}  |  j/k navigate  enter set wallpaper  / filter  q quit")
        } else {
            " j/k navigate  enter set wallpaper  / filter  q quit".to_string()
        };

        frame.render_widget(Line::from(text).fg(MUTED), area);
    }
}

fn preview_block(date_title: &str) -> Block<'_> {
    Block::bordered()
        .border_style(Style::new().fg(BORDER))
        .border_type(BorderType::Rounded)
        .title(Span::from(format!(" {date_title} ")).fg(MUTED).italic())
}

fn format_date(date_str: &str) -> String {
    chrono::DateTime::parse_from_rfc2822(date_str)
        .map(|dt| dt.format("%B %-d, %Y").to_string())
        .unwrap_or_default()
}
