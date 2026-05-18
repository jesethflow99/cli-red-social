use anyhow::Result;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io::{Read, Stdout, Write};
use std::panic;

use crate::db::DatabaseOps;
use crate::i18n::{self, Lang};
use crate::t;
use crate::models::{Comment, Message, Notification, Post, Screen, User};
use crate::theme::AppTheme;

pub fn run_tui(db_conn: &str) -> Result<()> {
    let database = crate::db::Database::new(db_conn)?;
    let app_db: Box<dyn DatabaseOps> = Box::new(database);

    let _terminal_guard = TerminalGuard::enter()?;
    let stdout = std::io::stdout();
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    let mut app = App::new(app_db, Lang::from_env());
    let result = app.run(&mut terminal);
    let _ = terminal.show_cursor();
    result
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        let _ = crossterm::execute!(
            stdout,
            crossterm::terminal::EnterAlternateScreen,
            crossterm::cursor::Hide,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        );
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let mut stdout = std::io::stdout();
        let _ = crossterm::execute!(
            stdout,
            crossterm::cursor::Show,
            crossterm::terminal::LeaveAlternateScreen,
        );
    }
}

pub struct App {
    pub db: Box<dyn DatabaseOps>,
    pub lang: Lang,
    pub theme: AppTheme,
    pub current_user: Option<User>,
    pub screen: Screen,
    pub timeline: Vec<Post>,
    pub input: String,
    pub status_message: Option<String>,
    pub debug_message: Option<String>,
    pub list_state: ListState,
    pub search_results: Vec<User>,
    pub viewed_user: Option<User>,
    pub viewed_user_posts: Vec<Post>,
    pub is_following_viewed: bool,
    pub attached_image: Option<String>,
    pub viewed_post: Option<Post>,
    pub post_comments: Vec<Comment>,
    pub comment_input: String,
    pub needs_clear: bool,
    pub comment_mode: bool,
    pub edit_mode: bool,
    pub edit_buffer: String,
    pub comment_list_state: ListState,
    pub url_mode: bool,
    pub profile_followers: Vec<User>,
    pub profile_following: Vec<User>,
    pub show_follow_list: bool,
    pub show_followers: bool,
    pub confirming_delete: bool,
    pub saved_post_input: String,
    pub conversations: Vec<User>,
    pub chat_messages: Vec<Message>,
    pub chat_partner: Option<User>,
    pub unread_count: i64,
    pub notifications: Vec<Notification>,
    pub unread_notifications: i64,
    pub profile_display_name: String,
    pub profile_bio: String,
    pub edit_profile_focus: usize,
    pub post_search_results: Vec<Post>,
    pub post_search_filter: &'static str,
    pub page: usize,
    pub page_size: usize,
    pub frame_count: u64,
}

const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

impl App {
    fn spinner_char(&self) -> char {
        SPINNER[(self.frame_count as usize / 3) % SPINNER.len()]
    }

    pub fn new(db: Box<dyn DatabaseOps>, lang: Lang) -> Self {
        Self {
            db,
            lang,
            theme: AppTheme::default(),
            current_user: None,
            screen: Screen::Login,
            timeline: vec![],
            input: String::new(),
            status_message: None,
            debug_message: None,
            list_state: ListState::default(),
            search_results: vec![],
            viewed_user: None,
            viewed_user_posts: vec![],
            is_following_viewed: false,
            attached_image: None,
            viewed_post: None,
            post_comments: vec![],
            comment_input: String::new(),
            needs_clear: false,
            comment_mode: false,
            edit_mode: false,
            edit_buffer: String::new(),
            comment_list_state: ListState::default(),
            url_mode: false,
            profile_followers: vec![],
            profile_following: vec![],
            show_follow_list: false,
            show_followers: false,
            confirming_delete: false,
            saved_post_input: String::new(),
            conversations: vec![],
            chat_messages: vec![],
            chat_partner: None,
            unread_count: 0,
            notifications: vec![],
            unread_notifications: 0,
            profile_display_name: String::new(),
            profile_bio: String::new(),
            edit_profile_focus: 0,
            post_search_results: vec![],
            post_search_filter: "all",
            page: 0,
            page_size: 20,
            frame_count: 0,
        }
    }

    fn draw_safe(
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        app: &App,
    ) -> std::result::Result<(), std::io::Error> {
        terminal.draw(|f| app.render(f))?;
        Ok(())
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        loop {
            self.frame_count += 1;

            if self.needs_clear {
                terminal.clear()?;
                self.needs_clear = false;
            }

            let draw_result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                Self::draw_safe(terminal, self).ok()
            }));
            match draw_result {
                Ok(Some(())) => {
                    self.clear_debug();
                }
                Ok(None) => {
                    let msg = "Error de render".to_string();
                    tracing::error!("{msg}");
                    self.set_status("Error interno de UI. Reintentando...".into());
                    self.set_debug(msg);
                    self.screen = Screen::Login;
                }
                Err(_) => {
                    let msg = "Panic during TUI render".to_string();
                    tracing::error!("{msg}");
                    self.set_status("Error interno en la UI. Reintentando...".into());
                    self.set_debug(msg);
                    self.screen = Screen::Login;
                }
            }

            let mut should_break = false;
            let event_result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                if event::poll(std::time::Duration::from_millis(80)).unwrap_or(false) {
                    if let Ok(Event::Key(key)) = event::read() {
                        if key.kind == KeyEventKind::Press {
                            match self.handle_key(key) {
                                Ok(false) => should_break = true,
                                Ok(true) => {}
                                Err(err) => {
                                    tracing::error!("TUI action failed on {:?}: {err}", self.screen);
                                    self.set_status(format!("Error: {err}"));
                                }
                            }
                        }
                    }
                }
            }));
            match event_result {
                Ok(()) => {}
                Err(_) => {
                    let msg = "Panic in TUI event handler".to_string();
                    tracing::error!("{msg}");
                    self.set_status("Error interno. Reintentando...".into());
                    self.screen = Screen::Login;
                }
            }
            if should_break {
                break;
            }
        }
        Ok(())
    }

    fn set_status(&mut self, msg: String) {
        self.status_message = Some(msg);
    }

    fn set_debug(&mut self, msg: String) {
        self.debug_message = Some(msg);
    }

    fn clear_debug(&mut self) {
        self.debug_message = None;
    }

    fn handle_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        match self.screen {
            Screen::Login => self.handle_login_key(key),
            Screen::Register => self.handle_register_key(key),
            Screen::Timeline => self.handle_timeline_key(key),
            Screen::CreatePost => self.handle_create_post_key(key),
            Screen::PostDetail(_) => self.handle_post_detail_key(key),
            Screen::Profile(_) => self.handle_profile_key(key),
            Screen::UserSearch => self.handle_search_key(key),
            Screen::Messages => self.handle_messages_key(key),
            Screen::Chat(_) => self.handle_chat_key(key),
            Screen::EditProfile => self.handle_edit_profile_key(key),
            Screen::Notifications => self.handle_notifications_key(key),
            Screen::PostSearch => self.handle_post_search_key(key),
            Screen::PostSearchFilter => self.handle_post_search_filter_key(key),
        }
    }

    fn handle_login_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => return Ok(false),
            KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => return Ok(false),
            KeyCode::Char(c) => self.input.push(c),
            KeyCode::Backspace => { self.input.pop(); }
            KeyCode::Tab => {
                self.screen = Screen::Register;
                self.input.clear();
            }
            KeyCode::Enter => {
                let parts: Vec<&str> = self.input.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let username = parts[0].trim();
                    let password = parts[1].trim();
                    match self.db.authenticate(username, password)? {
                        Some(user) => {
                            self.current_user = Some(user);
                            self.screen = Screen::Timeline;
                            self.page = 0;
                            self.input.clear();
                            if let Err(err) = self.refresh_timeline() {
                                tracing::error!("refresh_timeline failed: {err}");
                                self.set_status(format!("Error al cargar timeline: {err}"));
                            }
                        }
                        None => self.set_status(t!(self, login_error_invalid).to_string()),
                    }
                } else {
                    self.set_status(t!(self, login_error_format).to_string());
                }
            }
            _ => {}
        }
        Ok(true)
    }

    fn handle_register_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => return Ok(false),
            KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => return Ok(false),
            KeyCode::Char(c) => self.input.push(c),
            KeyCode::Backspace => { self.input.pop(); }
            KeyCode::Tab => {
                self.screen = Screen::Login;
                self.input.clear();
            }
            KeyCode::Enter => {
                let parts: Vec<&str> = self.input.splitn(3, ':').collect();
                if parts.len() == 3 {
                    let username = parts[0].trim();
                    let password = parts[1].trim();
                    let display = parts[2].trim();
                    if username.is_empty() {
                        self.set_status(t!(self, register_error_username_empty).to_string());
                    } else if password.len() < 4 {
                        self.set_status(t!(self, register_error_password_short).to_string());
                    } else if display.is_empty() {
                        self.set_status(t!(self, register_error_name_empty).to_string());
                    } else if let Err(e) = self.db.check_register_rate_limit() {
                        self.set_status(format!("{}", e));
                    } else {
                        match self.db.register_user(username, password, display) {
                            Ok(user) => {
                                self.current_user = Some(user);
                                self.screen = Screen::Timeline;
                                self.page = 0;
                                self.input.clear();
                                if let Err(err) = self.refresh_timeline() {
                                    tracing::error!("refresh_timeline failed: {err}");
                                    self.set_status(format!("Error al cargar timeline: {err}"));
                                }
                            }
                            Err(e) => {
                                let msg = if e.to_string().contains("UNIQUE") || e.to_string().contains("unique") {
                                    t!(self, register_error_exists).replace("{}", username)
                                } else {
                                    format!("{}: {}", t!(self, error), e)
                                };
                                self.set_status(msg);
                            }
                        }
                    }
                } else {
                    self.set_status("Formato: usuario:contraseña:nombre".into());
                }
            }
            _ => {}
        }
        Ok(true)
    }

    fn handle_timeline_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        self.status_message = None;
        match key.code {
            KeyCode::Char('q') => return Ok(false),
            KeyCode::Char('/') => {
                self.input.clear();
                self.post_search_results.clear();
                self.screen = Screen::PostSearch;
            }
            KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
                self.page = 0;
                self.notifications = self.db.get_notifications(self.current_user.as_ref().unwrap().id, 0, 50)?;
                self.unread_notifications = 0;
                self.db.mark_notifications_read(self.current_user.as_ref().unwrap().id)?;
                self.screen = Screen::Notifications;
            }
            KeyCode::Char('n') => {
                self.screen = Screen::CreatePost;
                self.input.clear();
            }
            KeyCode::Char('m') => {
                self.load_conversations()?;
                self.screen = Screen::Messages;
            }
            KeyCode::Char('s') => {
                self.screen = Screen::UserSearch;
                self.input.clear();
                self.search_results.clear();
            }
            KeyCode::Char('p') => {
                let id = self.current_user.as_ref().unwrap().id;
                self.screen = Screen::Profile(id);
                self.load_profile(id)?;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let len = self.timeline.len();
                if len > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some(i.saturating_sub(1)));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let len = self.timeline.len();
                if len > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some((i + 1).min(len - 1)));
                }
            }
            KeyCode::Char('f') if key.modifiers == KeyModifiers::CONTROL => {
                self.page += 1;
                self.list_state.select(Some(0));
                self.refresh_timeline()?;
            }
            KeyCode::Char('b') if key.modifiers == KeyModifiers::CONTROL => {
                self.page = self.page.saturating_sub(1);
                self.list_state.select(Some(0));
                self.refresh_timeline()?;
            }
            KeyCode::Enter => {
                if let Some(i) = self.list_state.selected() {
                    if i < self.timeline.len() {
                        let post = &self.timeline[i];
                        self.viewed_post = Some(post.clone());
                        self.post_comments = self.db.get_comments(post.id)?;
                        self.comment_input.clear();
                        self.comment_list_state = ListState::default();
                        self.edit_mode = false;
                        self.screen = Screen::PostDetail(post.id);
                    }
                }
            }
            KeyCode::Char('i') => {
                let img = self.list_state.selected().and_then(|i| {
                    self.timeline.get(i).and_then(|p| p.image_path.clone())
                });
                if let Some(ref path) = img {
                    Self::view_image_with_chafa(path, self);
                }
            }
            KeyCode::Char('d') => {
                if let Some(i) = self.list_state.selected() {
                    if let Some(post) = self.timeline.get(i) {
                        if Self::post_has_image(post) {
                            let path = post.image_path.clone().unwrap();
                            Self::show_download_instructions(&path, self);
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(true)
    }

    fn handle_create_post_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        if self.url_mode {
            match key.code {
                KeyCode::Char(c) => self.input.push(c),
                KeyCode::Backspace => { self.input.pop(); }
                KeyCode::Enter => {
                    let url = self.input.trim().to_string();
                    if url.is_empty() {
                        self.set_status(t!(self, create_post_enter_url).to_string());
                    } else if Self::is_valid_image_url(&url) {
                        self.attached_image = Some(url.clone());
                        self.set_status(format!("URL: {}", url));
                        self.input.clear();
                        self.input.push_str(&self.saved_post_input);
                        self.saved_post_input.clear();
                        self.url_mode = false;
                    } else {
                        self.set_status(t!(self, create_post_invalid_url).to_string());
                        self.input.clear();
                    }
                }
                KeyCode::Esc => {
                    self.input.clear();
                    self.input.push_str(&self.saved_post_input);
                    self.saved_post_input.clear();
                    self.url_mode = false;
                }
                _ => {}
            }
            return Ok(true);
        }

        match key.code {
            KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                self.saved_post_input = self.input.clone();
                self.input.clear();
                self.url_mode = true;
                self.set_status(t!(self, create_post_attach_prompt).to_string());
            }
            KeyCode::Char(c) => self.input.push(c),
            KeyCode::Backspace => { self.input.pop(); }
            KeyCode::Esc => {
                    self.page = 0;
                    self.screen = Screen::Timeline;
                    self.input.clear();
                    self.attached_image = None;
                }
                KeyCode::Enter => {
                if !self.input.trim().is_empty() {
                    let user_id = self.current_user.as_ref().unwrap().id;
                    let content = self.input.trim().to_string();
                    let img = self.attached_image.clone();
                    self.db.create_post(user_id, &content, img.as_deref())?;
                    let status = if img.is_some() { t!(self, create_post_published_img) } else { t!(self, create_post_published) };
                    self.set_status(status.to_string());
                    self.input.clear();
                    self.attached_image = None;
                    self.page = 0;
                    self.screen = Screen::Timeline;
                    self.refresh_timeline()?;
                }
            }
            _ => {}
        }
        Ok(true)
    }

    fn is_valid_image_url(url: &str) -> bool {
        let lower = url.trim().to_lowercase();
        if !lower.starts_with("http://") && !lower.starts_with("https://") {
            return false;
        }
        if !Self::is_safe_url(url) {
            return false;
        }
        lower.rsplit_once('.').map(|(_, e)| {
            let ext = e.split('?').next().unwrap_or("")
                .split('#').next().unwrap_or("")
                .split('&').next().unwrap_or("");
            matches!(ext, "jpg" | "jpeg" | "png" | "gif" | "webp")
        }).unwrap_or(false)
    }

    fn is_safe_url(url: &str) -> bool {
        use std::net::ToSocketAddrs;
        let host = url.trim_start_matches("http://")
            .trim_start_matches("https://")
            .split('/').next()
            .and_then(|h| h.split(':').next())
            .unwrap_or("");
        if host.is_empty() { return false; }
        if let Ok(ip) = host.to_lowercase().as_str().parse::<std::net::IpAddr>() {
            match ip {
                std::net::IpAddr::V4(v4) => !v4.is_private() && !v4.is_loopback() && !v4.is_unspecified(),
                std::net::IpAddr::V6(v6) => !v6.is_loopback() && !v6.is_unspecified(),
            }
        } else {
            if let Ok(addrs) = (host, 80).to_socket_addrs() {
                for addr in addrs {
                    let ip = addr.ip();
                    match ip {
                        std::net::IpAddr::V4(v4) if v4.is_private() || v4.is_loopback() || v4.is_unspecified() => return false,
                        std::net::IpAddr::V6(v6) if v6.is_loopback() || v6.is_unspecified() => return false,
                        _ => {}
                    }
                }
                true
            } else {
                false
            }
        }
    }

    fn is_url(s: &str) -> bool {
        s.starts_with("http://") || s.starts_with("https://")
    }

    fn download_to_temp(url: &str) -> Option<String> {
        let url = url.to_string();
        let ext = url.rsplit_once('.').map(|(_, e)| {
            e.split('?').next().unwrap_or("jpg").split('#').next().unwrap_or("jpg")
        }).unwrap_or("jpg").to_string();
        let bytes = std::thread::spawn(move || {
            reqwest::blocking::get(&url).ok()?.bytes().ok()
        }).join().ok()??;
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
        let path = format!("/tmp/opencode_img_{}.{}", ts, ext);
        std::fs::write(&path, &bytes).ok()?;
        Some(path)
    }

    fn view_image(path: &str) -> bool {
        let (term_w, term_h) = crossterm::terminal::size().unwrap_or((80, 40));
        let img_w = 80.min(term_w.saturating_sub(2));
        let img_h = 40.min(term_h.saturating_sub(2));
        let x = (term_w - img_w) / 2;
        let y = (term_h - img_h) / 2;
        let place = format!("{}x{}@{}x{}", img_w, img_h, x, y);
        let img_w_s = img_w.to_string();
        let img_h_s = img_h.to_string();
        let chafa_size = format!("{}x{}", img_w / 2, img_h);

        let viewers: [(&str, &[&str]); 4] = [
            ("kitten", &["icat", "--place", &place, path]),
            ("viu", &["-w", &img_w_s, "-h", &img_h_s, path]),
            ("catimg", &["-w", &img_w_s, "-h", &img_h_s, path]),
            ("chafa", &["--symbols", "block", "--size", &chafa_size, path]),
        ];
        for (cmd, args) in &viewers {
            if let Ok(status) = std::process::Command::new(cmd).args(*args).status() {
                if status.success() {
                    return true;
                }
            }
        }
        false
    }

    fn view_image_with_chafa(path: &str, app: &mut App) {
        app.needs_clear = true;
        print!("\x1b[2J\x1b[H");
        println!("{}", t!(app, image_downloading));
        let _ = std::io::stdout().flush();

        let temp_path;
        let actual_path = if Self::is_url(path) {
            match Self::download_to_temp(path) {
                Some(p) => { temp_path = p; &temp_path }
                None => { println!("Error al descargar la imagen"); return; }
            }
        } else {
            path
        };

        print!("\x1b[2J\x1b[H");
        let _ = std::io::stdout().flush();

        let shown = Self::view_image(actual_path);

        if !shown {
            println!("{}", t!(app, image_no_viewer));
            println!("URL: {}", path);
        }

        println!("\n{}", t!(app, image_press_enter));
        let _ = std::io::stdout().flush();

        let stdin = std::io::stdin();
        let mut handle = stdin.lock();
        loop {
            let mut byte = [0u8];
            match handle.read(&mut byte) {
                Ok(_) if byte[0] == b'\r' || byte[0] == b'\n' => break,
                Err(_) => break,
                _ => continue,
            }
        }

        if Self::is_url(path) {
            let _ = std::fs::remove_file(actual_path);
        }
    }

    fn show_download_instructions(path: &str, app: &mut App) {
        app.needs_clear = true;
        print!("\x1b[2J\x1b[H");
        let _ = std::io::stdout().flush();

        println!("{}", t!(app, image_download_instructions));
        println!("{}", path);
        println!("\n{}", t!(app, image_open_browser));

        println!("\n{}", t!(app, image_press_enter));
        let _ = std::io::stdout().flush();

        let stdin = std::io::stdin();
        let mut handle = stdin.lock();
        loop {
            let mut byte = [0u8];
            match handle.read(&mut byte) {
                Ok(_) if byte[0] == b'\r' || byte[0] == b'\n' => break,
                Err(_) => break,
                _ => continue,
            }
        }
    }

    fn post_has_image(post: &Post) -> bool {
        post.image_path.as_ref().map_or(false, |p| !p.is_empty())
    }

    fn handle_post_detail_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        let user_id = self.current_user.as_ref().unwrap().id;

        if self.comment_mode {
            match key.code {
                KeyCode::Char(c) => self.comment_input.push(c),
                KeyCode::Backspace => { self.comment_input.pop(); }
                KeyCode::Enter => {
                    if !self.comment_input.trim().is_empty() {
                        if let Some(ref post) = self.viewed_post.clone() {
                            self.db.add_comment(post.id, user_id, self.comment_input.trim())?;
                            self.post_comments = self.db.get_comments(post.id)?;
                            self.comment_input.clear();
                        }
                    }
                    self.comment_mode = false;
                }
                KeyCode::Esc => {
                    self.comment_input.clear();
                    self.comment_mode = false;
                }
                _ => {}
            }
            return Ok(true);
        }

        if self.edit_mode {
            match key.code {
                KeyCode::Char(c) => self.edit_buffer.push(c),
                KeyCode::Backspace => { self.edit_buffer.pop(); }
                KeyCode::Enter => {
                    if !self.edit_buffer.trim().is_empty() {
                        if let Some(ref post) = self.viewed_post.clone() {
                            self.db.update_post(post.id, user_id, self.edit_buffer.trim())?;
                            self.viewed_post = Some(Post { content: self.edit_buffer.trim().to_string(), ..post.clone() });
                            self.set_status(t!(self, post_detail_edited).to_string());
                        }
                    }
                    self.edit_mode = false;
                }
                KeyCode::Esc => {
                    self.edit_mode = false;
                }
                _ => {}
            }
            return Ok(true);
        }

        match key.code {
            KeyCode::Char('b') | KeyCode::Esc => {
                self.screen = Screen::Timeline;
                self.comment_input.clear();
                self.refresh_timeline()?;
            }
            KeyCode::Char('i') => {
                let img = self.viewed_post.as_ref()
                    .and_then(|p| p.image_path.clone())
                    .filter(|p| !p.is_empty());
                if let Some(ref path) = img {
                    Self::view_image_with_chafa(path, self);
                }
            }
            KeyCode::Char('c') => {
                self.comment_mode = true;
            }
            KeyCode::Char('e') => {
                if let Some(ref post) = self.viewed_post {
                    if post.user_id == user_id {
                        self.edit_buffer = post.content.clone();
                        self.edit_mode = true;
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let len = self.post_comments.len();
                if len > 0 {
                    let i = self.comment_list_state.selected().unwrap_or(0);
                    self.comment_list_state.select(Some(i.saturating_sub(1)));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let len = self.post_comments.len();
                if len > 0 {
                    let i = self.comment_list_state.selected().unwrap_or(0);
                    self.comment_list_state.select(Some((i + 1).min(len - 1)));
                }
            }
            KeyCode::Char('d') => {
                if let Some(i) = self.comment_list_state.selected() {
                    if let Some(comment) = self.post_comments.get(i) {
                        if comment.user_id == user_id {
                            self.db.delete_comment(comment.id, user_id)?;
                            if let Some(ref post) = self.viewed_post {
                                self.post_comments = self.db.get_comments(post.id)?;
                            }
                            let n = self.post_comments.len();
                            if n == 0 {
                                self.comment_list_state.select(None);
                            } else {
                                self.comment_list_state.select(Some(i.min(n - 1)));
                            }
                            self.set_status(t!(self, post_detail_comment_deleted).to_string());
                        }
                    }
                }
            }
            KeyCode::Char('D') => {
                if let Some(ref post) = self.viewed_post.clone() {
                    if post.user_id == user_id {
                            self.db.delete_post(post.id, user_id)?;
                            self.set_status(t!(self, post_detail_deleted).to_string());
                        self.screen = Screen::Timeline;
                        self.refresh_timeline()?;
                    }
                }
            }
            _ => {}
        }
        Ok(true)
    }

    fn render_profile(&self, f: &mut Frame, area: Rect) {
        if let Some(ref user) = self.viewed_user {
            let current = self.current_user.as_ref().unwrap();

            if self.show_follow_list {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(1)])
                    .margin(1)
                    .split(area);

                let title = if self.show_followers {
                    t!(self, profile_title_followers).replace("{}", &self.profile_followers.len().to_string())
                } else {
                    t!(self, profile_title_following).replace("{}", &self.profile_following.len().to_string())
                };
                let list = if self.show_followers { &self.profile_followers } else { &self.profile_following };
                let selected = self.list_state.selected().unwrap_or(0);
                let total = list.len();
                let items: Vec<ListItem> = list.iter().enumerate().map(|(i, u)| {
                    let bullet = if total > 0 && i == selected { "\u{25b6} " } else { "  " };
                    ListItem::new(Line::from(vec![
                        Span::raw(bullet),
                        Span::styled(format!("@{}", u.username), Style::default().fg(self.theme.accent)),
                        Span::raw("  "),
                        Span::raw(&u.display_name),
                    ]))
                    .style(self.theme.list_item_style(i))
                }).collect();
                let list_widget = List::new(items)
                    .block(self.theme.default_block(&title))
                    .highlight_style(self.theme.highlight())
                    .highlight_symbol("  ");
                f.render_stateful_widget(list_widget, chunks[1], &mut self.list_state.clone());
                let help = Paragraph::new(Line::from(Span::styled(t!(self, profile_help_follow_list), Style::default().fg(self.theme.secondary))));
                f.render_widget(help, chunks[0]);
                return;
            }

            let fc = t!(self, profile_followers_count).replace("{}", &self.profile_followers.len().to_string());
            let fg = t!(self, profile_following_count).replace("{}", &self.profile_following.len().to_string());
            let pc = t!(self, profile_header_posts).replace("{}", &self.viewed_user_posts.len().to_string());
            let hf = t!(self, profile_header_fmt).replace("{}", &user.username).replace("{}", &user.display_name);
            let follow_status = if current.id == user.id {
                format!(" ({})", t!(self, profile_you))
            } else if self.is_following_viewed {
                format!(" [{}]", t!(self, profile_following))
            } else {
                format!(" [{}]", t!(self, profile_not_following))
            };
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(6), Constraint::Min(1), Constraint::Length(1)])
                .margin(1)
                .split(area);

            let header_text = format!("{}  |  {}{}\n{} {}", hf, pc, follow_status, fc, fg);
            let header = Paragraph::new(header_text)
                .style(Style::default().fg(self.theme.primary))
                .block(self.theme.simple_block());
            f.render_widget(header, chunks[0]);

            if self.confirming_delete {
                let input = Paragraph::new(self.input.chars().map(|_| '*').collect::<String>())
                    .style(Style::default().fg(self.theme.accent))
                    .block(Block::default().title(" Contraseña ").borders(Borders::ALL).border_type(BorderType::Rounded));
                f.render_widget(input, chunks[1]);
                Self::set_cursor_clamped(f, area.x + 2 + self.input.len() as u16, chunks[1].y + 1);
            } else {
                let selected = self.list_state.selected().unwrap_or(0);
                let total = self.viewed_user_posts.len();
                let items: Vec<ListItem> = self
                .viewed_user_posts
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let ago = i18n::ago(self.lang, &p.created_at, self.current_user.as_ref().map(|u| u.utc_offset).unwrap_or(0));
                    let img = if Self::post_has_image(p) { "  \u{1f4f7}" } else { "" };
                    let bullet = if total > 0 && i == selected { "\u{25b6} " } else { "  " };
                    ListItem::new(Line::from(vec![
                        Span::raw(bullet),
                        Span::raw(&p.content),
                        Span::styled(img, Style::default().fg(self.theme.image_indicator)),
                        Span::styled(format!("  [{}]", ago), Style::default().fg(self.theme.muted)),
                    ]))
                    .style(self.theme.list_item_style(i))
                })
                .collect();

                let title = format!("{}  [{}]", t!(self, profile_title_posts), total);
                let list = List::new(items)
                    .block(self.theme.default_block(&title))
                    .highlight_style(self.theme.highlight())
                    .highlight_symbol("  ");
                f.render_stateful_widget(list, chunks[1], &mut self.list_state.clone());
            }

            let help = Paragraph::new(Line::from(Span::styled(t!(self, profile_help), Style::default().fg(self.theme.secondary))));
            f.render_widget(help, chunks[2]);
        }
    }

    fn render_search(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .margin(1)
            .split(area);

        let input = Paragraph::new(self.input.as_str())
            .style(Style::default().fg(self.theme.text))
            .block(
                Block::default()
                    .title(t!(self, search_title))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            );
        f.render_widget(input, chunks[0]);
        f.set_cursor_position((
            area.x + 2 + self.input.len() as u16,
            area.y + 2,
        ));

        let selected = self.list_state.selected().unwrap_or(0);
        let total = self.search_results.len();
        let items: Vec<ListItem> = self
            .search_results
            .iter()
            .enumerate()
            .map(|(i, u)| {
                let bullet = if total > 0 && i == selected { "\u{25b6} " } else { "  " };
                ListItem::new(Line::from(vec![
                    Span::raw(bullet),
                    Span::styled(format!("@{}", u.username), Style::default().fg(self.theme.accent)),
                    Span::raw("  "),
                    Span::raw(&u.display_name),
                ]))
                .style(self.theme.list_item_style(i))
            })
            .collect();

        let title = format!("{}  [{}]", t!(self, search_results), total);
        let list = List::new(items)
            .block(self.theme.default_block(&title))
            .highlight_style(self.theme.highlight())
            .highlight_symbol("  ");
        f.render_stateful_widget(list, chunks[1], &mut self.list_state.clone());
    }

    fn render_post_search(&self, f: &mut Frame, area: Rect) {
        let filter_label = match self.post_search_filter {
            "24h" => t!(self, post_search_filter_24h),
            "7d" => t!(self, post_search_filter_7d),
            "30d" => t!(self, post_search_filter_30d),
            _ => t!(self, post_search_filter_all),
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(1), Constraint::Min(1)])
            .margin(1)
            .split(area);

        let title = t!(self, post_search_title).replace("{}", filter_label);
        let input = Paragraph::new(self.input.as_str())
            .style(Style::default().fg(self.theme.text))
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            );
        f.render_widget(input, chunks[0]);
        f.set_cursor_position((
            area.x + 2 + self.input.len() as u16,
            area.y + 2,
        ));

        let help = Paragraph::new(Line::from(Span::styled(t!(self, post_search_help), Style::default().fg(self.theme.secondary))));
        f.render_widget(help, chunks[1]);

        let selected = self.list_state.selected().unwrap_or(0);
        let total = self.post_search_results.len();
        let items: Vec<ListItem> = self
            .post_search_results
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let ago = i18n::ago(self.lang, &p.created_at, self.current_user.as_ref().map(|u| u.utc_offset).unwrap_or(0));
                let img = if p.image_path.is_some() { "  \u{1f4f7}" } else { "" };
                let bullet = if total > 0 && i == selected { "\u{25b6} " } else { "  " };
                ListItem::new(Line::from(vec![
                    Span::raw(bullet),
                    Span::styled(format!("@{}", p.username), Style::default().fg(self.theme.accent)),
                    Span::raw(": "),
                    Span::raw(&p.content),
                    Span::styled(img, Style::default().fg(self.theme.image_indicator)),
                    Span::styled(format!("  [{}]", ago), Style::default().fg(self.theme.muted)),
                ]))
                .style(self.theme.list_item_style(i))
            })
            .collect();

        let title = format!("{}  [{}]", t!(self, post_search_results), total);
        let list = List::new(items)
            .block(self.theme.default_block(&title))
            .highlight_style(self.theme.highlight())
            .highlight_symbol("  ");
        f.render_stateful_widget(list, chunks[2], &mut self.list_state.clone());
    }

    fn render_post_search_filter(&self, f: &mut Frame, area: Rect) {
        let current = self.post_search_filter;
        let items: Vec<ListItem> = [
            ("all", t!(self, post_search_filter_all)),
            ("24h", t!(self, post_search_filter_24h)),
            ("7d", t!(self, post_search_filter_7d)),
            ("30d", t!(self, post_search_filter_30d)),
        ]
        .iter()
        .map(|(k, label)| {
            let prefix = if *k == current { "✓ " } else { "  " };
            ListItem::new(Line::from(Span::raw(format!("{}{}", prefix, label))))
        })
        .collect();

        let list = List::new(items)
            .block(Block::default().title(t!(self, post_search_filter_title)).borders(Borders::ALL).border_type(BorderType::Rounded))
            .highlight_style(self.theme.highlight())
            .highlight_symbol("> ");
        let area = Layout::default()
            .horizontal_margin(2)
            .vertical_margin(5)
            .constraints([Constraint::Length(6)])
            .split(area)[0];
        f.render_widget(list, area);
    }

    fn handle_profile_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        if self.show_follow_list {
            match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let list = if self.show_followers { &self.profile_followers } else { &self.profile_following };
                let len = list.len();
                if len > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some(i.saturating_sub(1)));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let list = if self.show_followers { &self.profile_followers } else { &self.profile_following };
                let len = list.len();
                if len > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some((i + 1).min(len - 1)));
                }
            }
                KeyCode::Enter => {
                    let list = if self.show_followers { &self.profile_followers.clone() } else { &self.profile_following.clone() };
                    if let Some(i) = self.list_state.selected() {
                        if let Some(user) = list.get(i) {
                            self.show_follow_list = false;
                            self.screen = Screen::Profile(user.id);
                            return self.load_profile(user.id).map(|_| true);
                        }
                    }
                }
                KeyCode::Esc | KeyCode::Char('b') => {
                    self.show_follow_list = false;
                }
                _ => {}
            }
            return Ok(true);
        }
        let user_id = self.current_user.as_ref().unwrap().id;

        if self.confirming_delete {
            match key.code {
                KeyCode::Char(c) => {
                    self.input.push(c);
                }
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Enter => {
                    let username = self.current_user.as_ref().unwrap().username.clone();
                    match self.db.authenticate(&username, &self.input)? {
                        Some(_) => {
                            self.db.delete_user(user_id)?;
                            self.current_user = None;
                            self.screen = Screen::Login;
                            self.input.clear();
                            self.status_message = None;
                            self.confirming_delete = false;
                        }
                        None => {
                            self.set_status(t!(self, profile_wrong_password).to_string());
                            self.confirming_delete = false;
                            self.input.clear();
                        }
                    }
                }
                KeyCode::Esc => {
                    self.confirming_delete = false;
                    self.status_message = None;
                    self.input.clear();
                }
                _ => {}
            }
            return Ok(true);
        }

        match key.code {
            KeyCode::Char('b') | KeyCode::Esc => {
                self.page = 0;
                let offset = self.page as u64 * self.page_size as u64;
                self.timeline = self.db.get_timeline(user_id, offset, self.page_size as u64 + 1)?;
                self.screen = Screen::Timeline;
            }
            KeyCode::Char('w') => {
                self.show_follow_list = true;
                self.show_followers = true;
                self.list_state.select(Some(0));
            }
            KeyCode::Char('g') => {
                self.show_follow_list = true;
                self.show_followers = false;
                self.list_state.select(Some(0));
            }
            KeyCode::Char('f') => {
                let viewed_id = self.viewed_user.as_ref().map(|u| u.id);
                let viewed_username = self.viewed_user.as_ref().map(|u| u.username.clone());
                if let Some(id) = viewed_id {
                    if id != user_id {
                        if self.is_following_viewed {
                            self.db.unfollow_user(user_id, id)?;
                            self.is_following_viewed = false;
                            let name = viewed_username.unwrap_or_default();
                            self.set_status(t!(self, profile_unfollowed).replace("{}", &name));
                        } else {
                            self.db.follow_user(user_id, id)?;
                            self.is_following_viewed = true;
                            let name = viewed_username.unwrap_or_default();
                            self.set_status(t!(self, profile_followed).replace("{}", &name));
                            let _ = self.db.add_notification(id, user_id, "follow");
                        }
                            let offset = self.page as u64 * self.page_size as u64;
                            self.timeline = self.db.get_timeline(user_id, offset, self.page_size as u64 + 1)?;
                    }
                }
            }
            KeyCode::Char('e') => {
                if self.viewed_user.as_ref().map(|u| u.id) == Some(user_id) {
                    if let Some(ref u) = self.viewed_user {
                        self.profile_display_name = u.display_name.clone();
                        self.profile_bio = u.bio.clone();
                        self.edit_profile_focus = 0;
                        self.screen = Screen::EditProfile;
                        self.input.clear();
                    }
                }
            }
            KeyCode::Char('x') => {
                if let Some(ref viewed) = self.viewed_user {
                    if viewed.id == user_id {
                        self.confirming_delete = true;
                        self.input.clear();
                        self.set_status(t!(self, profile_delete_confirm).to_string());
                    }
                }
            }
            KeyCode::Char('m') => {
                if self.viewed_user.as_ref().map(|u| u.id) != Some(user_id) {
                    if let Some(ref viewed) = self.viewed_user {
                        let other_id = viewed.id;
                        self.load_chat(other_id)?;
                        self.screen = Screen::Chat(other_id);
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let len = self.viewed_user_posts.len();
                if len > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some(i.saturating_sub(1)));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let len = self.viewed_user_posts.len();
                if len > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some((i + 1).min(len - 1)));
                }
            }
            KeyCode::Enter => {
                if let Some(i) = self.list_state.selected() {
                    if let Some(post) = self.viewed_user_posts.get(i) {
                        self.viewed_post = Some(post.clone());
                        self.post_comments = self.db.get_comments(post.id)?;
                        self.comment_input.clear();
                        self.comment_list_state = ListState::default();
                        self.edit_mode = false;
                        self.screen = Screen::PostDetail(post.id);
                    }
                }
            }
            _ => {}
        }
        Ok(true)
    }

    fn handle_search_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Up => {
                let len = self.search_results.len();
                if len > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some(i.saturating_sub(1)));
                }
            }
            KeyCode::Down => {
                let len = self.search_results.len();
                if len > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some((i + 1).min(len - 1)));
                }
            }
            KeyCode::Char(c) => {
                if key.modifiers == KeyModifiers::CONTROL && c == 'f' {
                    self.page += 1;
                    let query = self.input.trim().to_string();
                    if !query.is_empty() {
                        let offset = self.page as u64 * self.page_size as u64;
                        self.search_results = self.db.search_users(&query, offset, self.page_size as u64)?;
                        self.list_state.select(Some(0));
                    }
                } else if key.modifiers == KeyModifiers::CONTROL && c == 'b' {
                    self.page = self.page.saturating_sub(1);
                    let query = self.input.trim().to_string();
                    if !query.is_empty() {
                        let offset = self.page as u64 * self.page_size as u64;
                        self.search_results = self.db.search_users(&query, offset, self.page_size as u64)?;
                        self.list_state.select(Some(0));
                    }
                } else {
                    self.input.push(c);
                }
            }
            KeyCode::Backspace => { self.input.pop(); }
            KeyCode::Enter => {
                if !self.search_results.is_empty() {
                    if let Some(i) = self.list_state.selected() {
                        if let Some(user) = self.search_results.get(i) {
                            self.screen = Screen::Profile(user.id);
                            self.input.clear();
                            return self.load_profile(user.id).map(|_| true);
                        }
                    }
                }
                let query = self.input.trim().to_string();
                if !query.is_empty() {
                    self.page = 0;
                    let offset = self.page as u64 * self.page_size as u64;
                    self.search_results = self.db.search_users(&query, offset, self.page_size as u64)?;
                    self.list_state.select(Some(0));
                }
            }
            KeyCode::Esc => {
                self.screen = Screen::Timeline;
                self.input.clear();
            }
            _ => {}
        }
        Ok(true)
    }

    fn handle_post_search_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.screen = Screen::Timeline;
                self.input.clear();
            }
            KeyCode::Char(c) => {
                if key.modifiers == KeyModifiers::CONTROL && c == 'f' {
                    self.page += 1;
                    let query = self.input.trim().to_string();
                    if !query.is_empty() {
                        let offset = self.page as u64 * self.page_size as u64;
                        self.post_search_results = self.db.search_posts(&query, self.post_search_filter, offset, self.page_size as u64)?;
                        self.list_state.select(Some(0));
                    }
                } else if key.modifiers == KeyModifiers::CONTROL && c == 'b' {
                    self.page = self.page.saturating_sub(1);
                    let query = self.input.trim().to_string();
                    if !query.is_empty() {
                        let offset = self.page as u64 * self.page_size as u64;
                        self.post_search_results = self.db.search_posts(&query, self.post_search_filter, offset, self.page_size as u64)?;
                        self.list_state.select(Some(0));
                    }
                } else {
                    self.input.push(c);
                }
            }
            KeyCode::Backspace => { self.input.pop(); }
            KeyCode::Tab => {
                self.screen = Screen::PostSearchFilter;
            }
            KeyCode::Up => {
                let len = self.post_search_results.len();
                if len > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some(i.saturating_sub(1)));
                }
            }
            KeyCode::Down => {
                let len = self.post_search_results.len();
                if len > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some((i + 1).min(len - 1)));
                }
            }
            KeyCode::Enter => {
                if self.list_state.selected().is_some() && !self.post_search_results.is_empty() {
                    if let Some(i) = self.list_state.selected() {
                        if let Some(post) = self.post_search_results.get(i) {
                            self.viewed_post = Some(post.clone());
                            self.post_comments = self.db.get_comments(post.id)?;
                            self.comment_input.clear();
                            self.comment_list_state = ListState::default();
                            self.edit_mode = false;
                            self.screen = Screen::PostDetail(post.id);
                        }
                    }
                } else {
                    let query = self.input.trim().to_string();
                    if !query.is_empty() {
                        self.page = 0;
                        let offset = self.page as u64 * self.page_size as u64;
                        self.post_search_results = self.db.search_posts(&query, self.post_search_filter, offset, self.page_size as u64)?;
                        self.list_state.select(Some(0));
                    }
                }
            }
            _ => {}
        }
        Ok(true)
    }

    fn handle_post_search_filter_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc | KeyCode::Tab => {
                self.screen = Screen::PostSearch;
            }
            KeyCode::Char('1') => self.post_search_filter = "all",
            KeyCode::Char('2') => self.post_search_filter = "24h",
            KeyCode::Char('3') => self.post_search_filter = "7d",
            KeyCode::Char('4') => self.post_search_filter = "30d",
            _ => {}
        }
        Ok(true)
    }

    fn refresh_timeline(&mut self) -> Result<()> {
        let user_id = self.current_user.as_ref().unwrap().id;
        let offset = self.page as u64 * self.page_size as u64;
        self.timeline = self.db.get_timeline(user_id, offset, self.page_size as u64 + 1)?;
        self.unread_count = self.db.get_unread_count(user_id)?;
        self.unread_notifications = self.db.get_unread_notifications_count(user_id)?;
        Ok(())
    }

    fn load_profile(&mut self, profile_id: i64) -> Result<()> {
        let user = self.db.get_user_by_id(profile_id)?.ok_or_else(|| anyhow::anyhow!("Usuario no encontrado"))?;
        let offset = self.page as u64 * self.page_size as u64;
        let posts = self.db.get_posts_by_user(profile_id, offset, self.page_size as u64 + 1)?;
        let current_id = self.current_user.as_ref().unwrap().id;
        let following = self.db.is_following(current_id, profile_id)?;
        self.viewed_user = Some(user);
        self.viewed_user_posts = posts;
        self.is_following_viewed = following;
        self.profile_followers = self.db.get_followers(profile_id)?;
        self.profile_following = self.db.get_following(profile_id)?;
        self.show_follow_list = false;
        Ok(())
    }

    fn load_conversations(&mut self) -> Result<()> {
        let user_id = self.current_user.as_ref().unwrap().id;
        self.conversations = self.db.get_conversations(user_id)?;
        self.unread_count = self.db.get_unread_count(user_id)?;
        self.list_state.select(Some(0));
        Ok(())
    }

    fn load_chat(&mut self, other_id: i64) -> Result<()> {
        let user_id = self.current_user.as_ref().unwrap().id;
        self.chat_partner = self.db.get_user_by_id(other_id)?;
        self.chat_messages = self.db.get_messages(user_id, other_id)?;
        self.db.mark_messages_read(user_id, other_id)?;
        self.input.clear();
        self.unread_count = self.db.get_unread_count(user_id)?;
        Ok(())
    }

    fn handle_messages_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        let user_id = self.current_user.as_ref().unwrap().id;
        self.conversations = self.db.get_conversations(user_id)?;
        self.unread_count = self.db.get_unread_count(user_id)?;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let len = self.conversations.len();
                if len > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some(i.saturating_sub(1)));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let len = self.conversations.len();
                if len > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some((i + 1).min(len - 1)));
                }
            }
            KeyCode::Enter => {
                if let Some(i) = self.list_state.selected() {
                    if let Some(user) = self.conversations.get(i) {
                        let user_id = user.id;
                        self.load_chat(user_id)?;
                        self.screen = Screen::Chat(user_id);
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('b') => {
                self.screen = Screen::Timeline;
            }
            KeyCode::Char('q') => return Ok(false),
            _ => {}
        }
        Ok(true)
    }

    fn handle_chat_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        let user_id = self.current_user.as_ref().unwrap().id;
        let partner_id = self.chat_partner.as_ref().map(|u| u.id).unwrap_or(0);
        self.chat_messages = self.db.get_messages(user_id, partner_id)?;
        self.unread_count = self.db.get_unread_count(user_id)?;
        match key.code {
            KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => return Ok(false),
            KeyCode::Char(c) => self.input.push(c),
            KeyCode::Backspace => { self.input.pop(); }
            KeyCode::Enter => {
                if !self.input.trim().is_empty() {
                    let partner_id = self.chat_partner.as_ref().map(|u| u.id).unwrap_or(0);
                    let msg = self.db.send_message(user_id, partner_id, self.input.trim())?;
                    self.chat_messages.push(msg);
                    self.input.clear();
                    self.unread_count = self.db.get_unread_count(user_id)?;
                }
            }
            KeyCode::Esc => {
                self.load_conversations()?;
                self.screen = Screen::Messages;
            }
            _ => {}
        }
        Ok(true)
    }

    fn handle_edit_profile_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Tab => {
                self.edit_profile_focus = (self.edit_profile_focus + 1) % 3;
            }
            KeyCode::Up => {
                if self.edit_profile_focus == 2 {
                    if let Some(ref mut u) = self.current_user {
                        u.utc_offset = (u.utc_offset + 30).min(840);
                    }
                }
            }
            KeyCode::Down => {
                if self.edit_profile_focus == 2 {
                    if let Some(ref mut u) = self.current_user {
                        u.utc_offset = (u.utc_offset - 30).max(-720);
                    }
                }
            }
            KeyCode::Char(c) => {
                if self.edit_profile_focus == 0 {
                    self.profile_display_name.push(c);
                } else if self.edit_profile_focus == 1 {
                    self.profile_bio.push(c);
                }
            }
            KeyCode::Backspace => {
                if self.edit_profile_focus == 0 {
                    self.profile_display_name.pop();
                } else if self.edit_profile_focus == 1 {
                    self.profile_bio.pop();
                }
            }
            KeyCode::Enter => {
                let user_id = self.current_user.as_ref().unwrap().id;
                let utc_offset = self.current_user.as_ref().unwrap().utc_offset;
                if self.profile_display_name.trim().is_empty() {
                    self.set_status(t!(self, profile_empty_name).to_string());
                } else {
                    self.db.update_profile(user_id, self.profile_display_name.trim(), self.profile_bio.trim(), utc_offset)?;
                    self.set_status(t!(self, profile_updated).to_string());
                    if let Some(ref mut u) = self.viewed_user {
                        u.display_name = self.profile_display_name.trim().to_string();
                        u.bio = self.profile_bio.trim().to_string();
                        u.utc_offset = utc_offset;
                    }
                    self.screen = Screen::Profile(user_id);
                }
            }
            KeyCode::Esc => {
                let user_id = self.current_user.as_ref().unwrap().id;
                self.screen = Screen::Profile(user_id);
            }
            _ => {}
        }
        Ok(true)
    }

    fn handle_notifications_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('b') => {
                self.unread_notifications = 0;
                self.screen = Screen::Timeline;
            }
            KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => return Ok(false),
            _ => {}
        }
        Ok(true)
    }

    fn render_messages(&self, f: &mut Frame, area: Rect) {
        let user = self.current_user.as_ref().unwrap();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)])
            .margin(1)
            .split(area);

            let header_text: String = if self.conversations.is_empty() {
                t!(self, messages_empty).to_string()
            } else {
                t!(self, messages_title).replace("{}", &user.username)
            };
        let header = Paragraph::new(header_text)
            .style(self.theme.header_style);
        f.render_widget(header, chunks[0]);

        let selected = self.list_state.selected().unwrap_or(0);
        let total = self.conversations.len();
        let items: Vec<ListItem> = self.conversations.iter().enumerate().map(|(i, u)| {
            let bullet = if total > 0 && i == selected { "\u{25b6} " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::raw(bullet),
                Span::styled(format!("@{}", u.username), Style::default().fg(self.theme.accent)),
                Span::raw(" \u{2014} "),
                Span::styled(&u.display_name, Style::default().fg(self.theme.secondary)),
            ]))
            .style(self.theme.list_item_style(i))
        }).collect();

        let title = format!("{}  [{}]", t!(self, messages_conversations), total);
        let list = List::new(items)
            .block(self.theme.default_block(&title))
            .highlight_style(self.theme.highlight())
            .highlight_symbol("  ");
        f.render_stateful_widget(list, chunks[1], &mut self.list_state.clone());

        let help = Paragraph::new(Line::from(Span::styled(t!(self, messages_help), Style::default().fg(self.theme.secondary))));
        f.render_widget(help, chunks[2]);
    }

    fn render_notifications(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .margin(1)
            .split(area);

        let header = Paragraph::new(t!(self, notifications_title))
            .style(self.theme.header_style);
        f.render_widget(header, chunks[0]);

        if self.notifications.is_empty() {
            let empty = Paragraph::new(t!(self, notifications_empty))
                .style(Style::default().fg(self.theme.muted));
            f.render_widget(empty, chunks[1]);
        } else {
            let offset = self.current_user.as_ref().map(|u| u.utc_offset).unwrap_or(0);
            let items: Vec<ListItem> = self.notifications.iter().enumerate().map(|(i, n)| {
                let msg = match n.notif_type.as_str() {
                    "follow" => t!(self, follow_notif).replace("{}", &n.from_username),
                    _ => format!("@{}: {}", n.from_username, n.notif_type),
                };
                let ago = i18n::ago(self.lang, &n.created_at, offset);
                let style = if n.read { Style::default().fg(self.theme.muted) } else { Style::default().fg(self.theme.text) };
                let unread = if !n.read { "\u{25cf} " } else { "  " };
                ListItem::new(Line::from(vec![
                    Span::styled(unread, Style::default().fg(self.theme.accent)),
                    Span::styled(msg, style),
                    Span::styled(format!("  [{}]", ago), Style::default().fg(self.theme.muted)),
                ]))
                .style(self.theme.list_item_style(i))
            }).collect();
            let title = format!("{}  [{}]", t!(self, notifications_title), self.notifications.len());
            let list = List::new(items)
                .block(self.theme.default_block(&title));
            f.render_widget(list, chunks[1]);
        }
    }

    fn render_edit_profile(&self, f: &mut Frame, area: Rect) {
        let user = self.current_user.as_ref().unwrap();
        let tz_offset = self.current_user.as_ref().map(|u| u.utc_offset).unwrap_or(0);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .margin(1)
            .split(area);

        let header = Paragraph::new(t!(self, edit_profile_title).replace("{}", &user.username))
            .style(self.theme.header_style);
        f.render_widget(header, chunks[0]);

        let name_style = if self.edit_profile_focus == 0 {
            Style::default().fg(self.theme.accent)
        } else {
            Style::default().fg(self.theme.text)
        };
        let name_input = Paragraph::new(self.profile_display_name.as_str())
            .style(name_style)
            .block(Block::default().title(t!(self, edit_profile_name)).borders(Borders::ALL).border_type(BorderType::Rounded));
        f.render_widget(name_input, chunks[1]);
        if self.edit_profile_focus == 0 {
            Self::set_cursor_clamped(f, chunks[1].x + 2 + self.profile_display_name.len() as u16, chunks[1].y + 1);
        }

        let bio_style = if self.edit_profile_focus == 1 {
            Style::default().fg(self.theme.accent)
        } else {
            Style::default().fg(self.theme.text)
        };
        let bio_input = Paragraph::new(self.profile_bio.as_str())
            .style(bio_style)
            .block(Block::default().title(t!(self, edit_profile_bio)).borders(Borders::ALL).border_type(BorderType::Rounded));
        f.render_widget(bio_input, chunks[2]);
        if self.edit_profile_focus == 1 {
            Self::set_cursor_clamped(f, chunks[2].x + 2 + self.profile_bio.len() as u16, chunks[2].y + 1);
        }

        let tz_title = t!(self, edit_profile_tz).replace("{}", &if tz_offset >= 0 { format!("+{}", tz_offset / 60) } else { format!("{}", tz_offset / 60) });
        let tz_style = if self.edit_profile_focus == 2 {
            Style::default().fg(self.theme.accent)
        } else {
            Style::default().fg(self.theme.text)
        };
        let tz_input = Paragraph::new(format!("{:+.1}", tz_offset as f64 / 60.0))
            .style(tz_style)
            .block(Block::default().title(tz_title).borders(Borders::ALL).border_type(BorderType::Rounded));
        f.render_widget(tz_input, chunks[3]);

        let help = Paragraph::new(Line::from(Span::styled(t!(self, edit_profile_help), Style::default().fg(self.theme.secondary))));
        f.render_widget(help, chunks[4]);
    }

    fn render_chat(&self, f: &mut Frame, area: Rect) {
        let user = self.current_user.as_ref().unwrap();
        let partner = self.chat_partner.as_ref().map(|u| u.username.as_str()).unwrap_or("...");
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(3),
                Constraint::Length(3),
            ])
            .margin(1)
            .split(area);

        let header = Paragraph::new(t!(self, chat_header).replace("{}", partner))
            .style(self.theme.header_style);
        f.render_widget(header, chunks[0]);

        let offset = self.current_user.as_ref().map(|u| u.utc_offset).unwrap_or(0);
        let messages: Vec<ListItem> = self.chat_messages.iter().enumerate().map(|(i, m)| {
            let style = if m.sender_id == user.id {
                Style::default().fg(self.theme.success)
            } else {
                Style::default().fg(self.theme.text)
            };
            let ago = i18n::ago(self.lang, &m.created_at, offset);
            ListItem::new(Line::from(vec![
                Span::styled(format!("@{}: ", m.sender_username), Style::default().fg(self.theme.accent)),
                Span::styled(&m.content, style),
                Span::styled(format!("  [{}]", ago), Style::default().fg(self.theme.muted)),
            ]))
            .style(self.theme.list_item_style(i))
        }).collect();

        let list = List::new(messages)
            .block(self.theme.simple_block());
        f.render_widget(list, chunks[1]);

        let input = Paragraph::new(self.input.as_str())
            .style(Style::default().fg(self.theme.text))
            .block(Block::default().title(t!(self, chat_input_title)).borders(Borders::ALL).border_type(BorderType::Rounded));
        f.render_widget(input, chunks[2]);
        Self::set_cursor_clamped(f, area.x + 2 + self.input.len() as u16, chunks[2].y + 1);
    }

    fn render(&self, f: &mut Frame) {
        let area = f.area();
        if self.current_user.is_some() {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(area);
            let content_area = chunks[0];
            self.render_status_bar(f, chunks[1]);
            match self.screen {
                Screen::Login => self.render_login(f, content_area),
                Screen::Register => self.render_register(f, content_area),
                Screen::Timeline => self.render_timeline(f, content_area),
                Screen::CreatePost => self.render_create_post(f, content_area),
                Screen::PostDetail(_) => self.render_post_detail(f, content_area),
                Screen::Profile(_) => self.render_profile(f, content_area),
                Screen::UserSearch => self.render_search(f, content_area),
                Screen::Messages => self.render_messages(f, content_area),
                Screen::Chat(_) => self.render_chat(f, content_area),
                Screen::EditProfile => self.render_edit_profile(f, content_area),
                Screen::Notifications => self.render_notifications(f, content_area),
                Screen::PostSearch => self.render_post_search(f, content_area),
                Screen::PostSearchFilter => self.render_post_search_filter(f, content_area),
            }
        } else {
            match self.screen {
                Screen::Login => self.render_login(f, area),
                Screen::Register => self.render_register(f, area),
                _ => self.render_login(f, area),
            }
        }
    }

    fn set_cursor_clamped(f: &mut Frame, x: u16, y: u16) {
        let area = f.area();
        let x = x.min(area.x + area.width.saturating_sub(1));
        let y = y.min(area.y + area.height.saturating_sub(1));
        f.set_cursor_position((x, y));
    }

    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let user = self.current_user.as_ref().unwrap();
        let t = &self.theme;

        let notif = if self.unread_notifications > 0 {
            format!(" \u{1f514}{}", self.unread_notifications)
        } else {
            String::new()
        };
        let msgs = if self.unread_count > 0 {
            format!(" \u{2709}{}", self.unread_count)
        } else {
            String::new()
        };

        let screen_name = match self.screen {
            Screen::Timeline => t!(self, status_bar_timeline),
            Screen::CreatePost => t!(self, status_bar_new_post),
            Screen::PostDetail(_) => t!(self, status_bar_post),
            Screen::Profile(_) => t!(self, status_bar_profile),
            Screen::UserSearch => t!(self, status_bar_search),
            Screen::Messages => t!(self, status_bar_messages),
            Screen::Chat(_) => t!(self, status_bar_chat),
            Screen::EditProfile => t!(self, status_bar_edit),
            Screen::Notifications => t!(self, status_bar_notifications),
            Screen::PostSearch | Screen::PostSearchFilter => t!(self, status_bar_post_search),
            Screen::Login | Screen::Register => "",
        };

        let scroll = if self.page > 0 {
            format!(" \u{25c0} p.{} \u{25b6}", self.page + 1)
        } else {
            String::new()
        };

        let spinner = self.spinner_char();

        let left = format!(" {}  {}  {}", screen_name, scroll, spinner);
        let right = format!(" @{} {}{}", user.username, msgs, notif);

        let bar = Paragraph::new(Line::from(vec![
            Span::styled(left, Style::default().fg(t.secondary)),
            Span::raw("  "),
            Span::styled(right, Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
        ]))
        .style(t.status_bar())
        .alignment(Alignment::Left);
        f.render_widget(bar, area);
    }

    fn render_login(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(1), Constraint::Length(1), Constraint::Min(1)])
            .margin(2)
            .split(area);

        let input = Paragraph::new(self.input.as_str())
            .style(Style::default().fg(self.theme.text))
            .block(Block::default().title(t!(self, login_title)).borders(Borders::ALL).border_type(BorderType::Rounded));
        f.render_widget(input, chunks[0]);
        Self::set_cursor_clamped(f, area.x + 2 + self.input.len() as u16, area.y + 3);

        if let Some(ref msg) = self.status_message {
            let status = Paragraph::new(msg.as_str())
                .style(Style::default().fg(self.theme.error));
            f.render_widget(status, chunks[1]);
        }

        if let Some(ref debug) = self.debug_message {
            let debug_par = Paragraph::new(debug.as_str())
                .style(Style::default().fg(self.theme.muted));
            f.render_widget(debug_par, chunks[2]);
        }

        let help = Paragraph::new(Line::from(Span::styled(t!(self, login_help), Style::default().fg(self.theme.secondary))));
        f.render_widget(help, chunks[3]);
    }

    fn render_register(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(1), Constraint::Length(1), Constraint::Min(1)])
            .margin(2)
            .split(area);

        let input = Paragraph::new(self.input.as_str())
            .style(Style::default().fg(self.theme.text))
            .block(Block::default().title(t!(self, register_title)).borders(Borders::ALL).border_type(BorderType::Rounded));
        f.render_widget(input, chunks[0]);
        Self::set_cursor_clamped(f, area.x + 2 + self.input.len() as u16, area.y + 3);

        if let Some(ref msg) = self.status_message {
            let status = Paragraph::new(msg.as_str())
                .style(Style::default().fg(self.theme.error));
            f.render_widget(status, chunks[1]);
        }

        if let Some(ref debug) = self.debug_message {
            let debug_par = Paragraph::new(debug.as_str())
                .style(Style::default().fg(self.theme.muted));
            f.render_widget(debug_par, chunks[2]);
        }

        let help = Paragraph::new(Line::from(Span::styled(t!(self, register_help), Style::default().fg(self.theme.secondary))));
        f.render_widget(help, chunks[3]);
    }

    fn render_timeline(&self, f: &mut Frame, area: Rect) {
        let user = self.current_user.as_ref().unwrap();
        let mut constraints = vec![
            Constraint::Length(3),
            Constraint::Length(0),
            Constraint::Length(1),
        ];
        let status_h = if self.status_message.is_some() { 1 } else { 0 };
        if status_h > 0 {
            constraints[1] = Constraint::Length(status_h);
        }
        constraints.push(Constraint::Min(5));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .margin(1)
            .split(area);

        let notif_indicator = if self.unread_notifications > 0 {
            format!("  \u{1f514}({})", self.unread_notifications)
        } else {
            String::new()
        };
        let page_info = if self.page > 0 {
            format!("  \u{25c0} {} {} \u{25b6}", t!(self, page), self.page + 1)
        } else {
            String::new()
        };
        let count_info = format!("  [{} {}]", self.timeline.len(), t!(self, status_bar_timeline));
        let header_str = format!("{} {}", &t!(self, timeline_title).replace("{}", &user.username), notif_indicator);
        let header = Paragraph::new(Line::from(vec![
            Span::styled(header_str, self.theme.header_style),
            Span::styled(count_info, Style::default().fg(self.theme.muted)),
            Span::styled(page_info, Style::default().fg(self.theme.accent)),
        ]));
        f.render_widget(header, chunks[0]);

        if let Some(ref msg) = self.status_message {
            let status = Paragraph::new(msg.as_str())
                .style(Style::default().fg(self.theme.success));
            f.render_widget(status, chunks[1]);
        }

        let selected = self.list_state.selected().unwrap_or(0);
        let total = self.timeline.len();
        let items: Vec<ListItem> = self.timeline.iter().enumerate().map(|(i, p)| {
            let ago = i18n::ago(self.lang, &p.created_at, self.current_user.as_ref().map(|u| u.utc_offset).unwrap_or(0));
            let img = if Self::post_has_image(p) { "  \u{1f4f7}" } else { "" };
            let bullet = if total > 0 && i == selected { "\u{25b6} " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::raw(bullet),
                Span::styled(format!("@{}", p.username), Style::default().fg(self.theme.accent)),
                Span::raw(": "),
                Span::raw(&p.content),
                Span::styled(img, Style::default().fg(self.theme.image_indicator)),
                Span::styled(format!("  [{}]", ago), Style::default().fg(self.theme.muted)),
            ]))
            .style(self.theme.list_item_style(i))
        }).collect();

        let timeline_chunk = chunks.len() - 1;
        let list = List::new(items)
            .block(self.theme.simple_block())
            .highlight_style(self.theme.highlight())
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[timeline_chunk], &mut self.list_state.clone());

        let help_idx = 2;
        let help = Paragraph::new(Line::from(Span::styled(t!(self, timeline_help), Style::default().fg(self.theme.secondary))));
        f.render_widget(help, chunks[help_idx]);
    }

    fn render_create_post(&self, f: &mut Frame, area: Rect) {
        let img_text = match self.attached_image {
            Some(ref path) => {
                let name = std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("imagen");
                format!("📷 {}", name)
            }
            None => String::new(),
        };

        let img_height = if img_text.is_empty() { 0 } else { 1 };
        let title = if self.url_mode { t!(self, create_post_url_title) } else { t!(self, create_post_title) };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(img_height),
                Constraint::Min(3),
            ])
            .margin(2)
            .split(area);

        let input = Paragraph::new(self.input.as_str())
            .style(Style::default().fg(self.theme.text))
            .block(Block::default().title(title).borders(Borders::ALL).border_type(BorderType::Rounded));
        f.render_widget(input, chunks[0]);
        Self::set_cursor_clamped(f, chunks[0].x + 2 + self.input.len() as u16, chunks[0].y + 1);

        if !img_text.is_empty() {
            let img_para = Paragraph::new(img_text.as_str())
                .style(Style::default().fg(self.theme.image_indicator));
            f.render_widget(img_para, chunks[1]);
        }

        let help = if self.url_mode {
            Paragraph::new(Line::from(Span::styled(t!(self, create_post_help_url), Style::default().fg(self.theme.secondary))))
        } else {
            Paragraph::new(Line::from(Span::styled(t!(self, create_post_help), Style::default().fg(self.theme.secondary))))
        };
        f.render_widget(help, chunks[chunks.len() - 1]);
    }

    fn render_post_detail(&self, f: &mut Frame, area: Rect) {
        if let Some(ref post) = self.viewed_post {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ])
                .margin(1)
                .split(area);

            let ago = i18n::ago(self.lang, &post.created_at, self.current_user.as_ref().map(|u| u.utc_offset).unwrap_or(0));
            let header = Paragraph::new(format!("@{}  [{}]", post.username, ago))
                .style(self.theme.header_style)
                .block(self.theme.simple_block());
            f.render_widget(header, chunks[0]);

            let mut inner = vec![
                Constraint::Length(1),  // post text
            ];
            if Self::post_has_image(post) {
                inner.push(Constraint::Length(5));  // image placeholder
            }
            inner.push(Constraint::Min(1));  // comments
            if self.comment_mode {
                inner.push(Constraint::Length(3));  // input
            } else if self.edit_mode {
                inner.push(Constraint::Length(3));  // edit input
            }

            let post_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(inner)
                .margin(1)
                .split(chunks[1]);

            let mut idx = 0;
            let text = Paragraph::new(post.content.as_str())
                .style(Style::default().fg(self.theme.text));
            f.render_widget(text, post_chunks[idx]); idx += 1;

            if Self::post_has_image(post) {
                let img_block = Block::default()
                    .title(t!(self, image_view))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(self.theme.image_indicator));
                let img_inner = img_block.inner(post_chunks[idx]);
                f.render_widget(img_block, post_chunks[idx]);
                let img_hint = Paragraph::new(t!(self, post_detail_image_hint))
                    .style(Style::default().fg(self.theme.muted))
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(img_hint, img_inner);
                idx += 1;
            }

            let comments: Vec<ListItem> = self.post_comments.iter().enumerate().map(|(i, c)| {
                let ago = i18n::ago(self.lang, &c.created_at, self.current_user.as_ref().map(|u| u.utc_offset).unwrap_or(0));
                ListItem::new(Line::from(vec![
                    Span::styled(format!("@{}", c.username), Style::default().fg(self.theme.accent)),
                    Span::raw(" "),
                    Span::raw(&c.content),
                    Span::styled(format!("  [{}]", ago), Style::default().fg(self.theme.muted)),
                ]))
                .style(self.theme.list_item_style(i))
            }).collect();

            let comments_list = List::new(if comments.is_empty() {
                vec![ListItem::new(Line::from(Span::styled(t!(self, post_detail_no_comments), Style::default().fg(self.theme.muted))))]
            } else {
                comments
            })
                .block(self.theme.default_block(t!(self, post_detail_comments)))
                .highlight_style(self.theme.highlight().add_modifier(Modifier::BOLD));
            let mut cs = self.comment_list_state.clone();
            f.render_stateful_widget(comments_list, post_chunks[idx], &mut cs); idx += 1;

            if self.edit_mode {
                let input = Paragraph::new(self.edit_buffer.as_str())
                    .style(Style::default().fg(self.theme.text))
                    .block(Block::default().title(t!(self, post_detail_edit_title)).borders(Borders::ALL).border_type(BorderType::Rounded));
                f.render_widget(input, post_chunks[idx]);
            } else if self.comment_mode {
                let input = Paragraph::new(self.comment_input.as_str())
                    .style(Style::default().fg(self.theme.text))
                    .block(Block::default().title(t!(self, post_detail_comment_title)).borders(Borders::ALL).border_type(BorderType::Rounded));
                f.render_widget(input, post_chunks[idx]);
            }

            let help = if self.edit_mode {
                Paragraph::new(Line::from(Span::styled(t!(self, post_detail_help_edit), Style::default().fg(self.theme.secondary))))
            } else if self.comment_mode {
                Paragraph::new(Line::from(Span::styled(t!(self, post_detail_help_comment), Style::default().fg(self.theme.secondary))))
            } else {
                Paragraph::new(Line::from(Span::styled(t!(self, post_detail_help_view), Style::default().fg(self.theme.secondary))))
            };
            f.render_widget(help, chunks[2]);
        }
    }
}
