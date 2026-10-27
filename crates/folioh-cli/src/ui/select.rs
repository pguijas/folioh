//! The init prompts: the arrow-key selector drawn with raw escapes and read
//! through crossterm raw mode, the numbered fallback when stdin or stdout is
//! not a terminal, and the news ticker that refreshes the banner's news line
//! in place while a prompt is open.

#[cfg(any(not(unix), test))]
use std::io::BufRead;
use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anstyle::Style;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::banner::{news_line, FOLIOH_NEWS_STYLE};
use super::text::{cell_len, paint, Colors, BOLD, BOLD_CYAN, DIM, RED};
use super::Ui;
use crate::signals::StopSignals;

/// SGR reset.
pub const RESET: &str = "\x1b[0m";
/// SGR bold.
pub const BOLD_SEQ: &str = "\x1b[1m";
/// SGR dim.
pub const DIM_SEQ: &str = "\x1b[2m";
/// SGR underline.
pub const UNDERLINE_SEQ: &str = "\x1b[4m";
/// 256-colour 147 (`#afafff`), the selector accent.
pub const ACCENT: &str = "\x1b[38;5;147m";
/// SGR green.
pub const GREEN_SEQ: &str = "\x1b[32m";
/// Erase the whole line.
pub const CLEAR_LINE: &str = "\x1b[2K";
/// Cursor up one row.
pub const CURSOR_UP: &str = "\x1b[1A";
/// Save the cursor position (DECSC).
pub const SAVE_CURSOR: &str = "\x1b7";
/// Restore the saved cursor position (DECRC).
pub const RESTORE_CURSOR: &str = "\x1b8";

/// The two init prompt titles; every title is padded to the widest.
pub const PROMPT_TITLES: [&str; 2] = ["Docstring style", "Visual preset"];

/// One selectable option: the value written to `docs.yaml`, the label the
/// menu shows, the description of the numbered fallback.
pub struct Choice {
    pub value: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

/// A key as the selector understands it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Enter,
    Cancel,
    Digit(usize),
    Other,
}

/// Why a prompt ended without an answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cancelled {
    /// The user left the prompt: Esc, `q`, Ctrl-C, or Ctrl-D on an empty line.
    ByUser,
    /// Stdin ended (or failed) before an answer arrived.
    InputClosed,
}

/// Arrow menus need a real terminal on both ends and no `CI`.
pub fn can_use_arrow_select() -> bool {
    std::env::var("CI").map_or(true, |v| v.is_empty())
        && io::stdin().is_terminal()
        && io::stdout().is_terminal()
}

fn title_width() -> usize {
    PROMPT_TITLES.iter().map(|t| cell_len(t)).max().unwrap_or(0)
}

fn formatted_title(title: &str) -> String {
    let padding = " ".repeat(title_width().saturating_sub(cell_len(title)));
    format!("{BOLD_SEQ}{title}{RESET}{padding}")
}

/// The open-menu title row: `? Title › - Use arrow-keys. Return to submit.`
pub fn prompt_line(title: &str) -> String {
    format!(
        "{ACCENT}?{RESET} {} {ACCENT}›{RESET} {DIM_SEQ}- Use arrow-keys. Return to submit.{RESET}",
        formatted_title(title)
    )
}

/// The row left behind after a pick: `✔ Title › value`.
pub fn completed_line(title: &str, value: &str) -> String {
    format!(
        "{GREEN_SEQ}✔{RESET} {} {ACCENT}›{RESET} {BOLD_SEQ}{value}{RESET}",
        formatted_title(title)
    )
}

/// One option row; the selected one carries the `›` cursor and an underline.
pub fn menu_line(label: &str, selected: bool) -> String {
    if selected {
        format!("› {ACCENT}{UNDERLINE_SEQ}{label}{RESET}")
    } else {
        format!("  {label}")
    }
}

/// Cursor bookkeeping the ticker needs to find the news line again.
pub struct TickerState {
    width: usize,
    rows_below_news: usize,
    active_prompt_lines: usize,
    last_line: String,
    colors: Colors,
}

impl TickerState {
    /// Lines printed permanently below the news line since the intro.
    pub fn add_static_lines(&mut self, count: usize) {
        self.rows_below_news += count;
    }

    /// Replace the count of currently drawn menu lines (prompt + options).
    pub fn set_active_prompt_lines(&mut self, count: usize) {
        self.rows_below_news = self.rows_below_news + count - self.active_prompt_lines;
        self.active_prompt_lines = count;
    }
}

/// Rewrites the banner's news line once per second while prompts are open.
/// The state mutex doubles as the terminal draw lock: the menu drawer holds
/// it while writing so a refresh never interleaves with a redraw.
pub struct NewsTicker {
    state: Arc<Mutex<TickerState>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl NewsTicker {
    /// Start refreshing the news line `rows_below_news` rows above the cursor.
    pub fn start(
        width: usize,
        rows_below_news: usize,
        initial_item: &str,
        colors: Colors,
    ) -> NewsTicker {
        let state = Arc::new(Mutex::new(TickerState {
            width,
            rows_below_news,
            active_prompt_lines: 0,
            last_line: news_line(Some(width), Some(initial_item)),
            colors,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let (thread_state, thread_stop) = (Arc::clone(&state), Arc::clone(&stop));
        let handle = thread::spawn(move || loop {
            // Wake once per second, but notice a stop request within a
            // tenth of that.
            for _ in 0..10 {
                if thread_stop.load(Ordering::Relaxed) {
                    return;
                }
                thread::sleep(Duration::from_millis(100));
            }
            Self::refresh(&thread_state);
        });
        NewsTicker {
            state,
            stop,
            handle: Some(handle),
        }
    }

    fn refresh(state: &Mutex<TickerState>) {
        let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
        let line = news_line(Some(state.width), None);
        if line == state.last_line {
            return;
        }
        state.last_line = line.clone();
        if state.rows_below_news < 1 {
            return;
        }
        let mut out = io::stdout().lock();
        let _ = write!(
            out,
            "{SAVE_CURSOR}\x1b[{}A\r{CLEAR_LINE}{}{RESTORE_CURSOR}",
            state.rows_below_news,
            paint(FOLIOH_NEWS_STYLE, &line, state.colors)
        );
        let _ = out.flush();
    }

    /// The draw lock; also the handle for the cursor bookkeeping.
    pub fn lock(&self) -> MutexGuard<'_, TickerState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Stop the thread and wait for it.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for NewsTicker {
    fn drop(&mut self) {
        self.stop();
    }
}

fn with_draw_lock<R>(
    ticker: Option<&NewsTicker>,
    f: impl FnOnce(Option<&mut TickerState>) -> R,
) -> R {
    match ticker {
        Some(ticker) => {
            let mut state = ticker.lock();
            f(Some(&mut state))
        }
        None => f(None),
    }
}

fn clear_menu(out: &mut dyn Write, ticker: Option<&NewsTicker>, line_count: usize) {
    with_draw_lock(ticker, |state| {
        let _ = out.write_all(
            format!("{CURSOR_UP}{CLEAR_LINE}")
                .repeat(line_count)
                .as_bytes(),
        );
        let _ = out.flush();
        if let Some(state) = state {
            state.set_active_prompt_lines(0);
        }
    });
}

fn accept(
    out: &mut dyn Write,
    ticker: Option<&NewsTicker>,
    title: &str,
    choice: &Choice,
    line_count: usize,
) -> &'static str {
    clear_menu(out, ticker, line_count);
    with_draw_lock(ticker, |state| {
        let _ = out.write_all(format!("{}\n", completed_line(title, choice.value)).as_bytes());
        let _ = out.flush();
        if let Some(state) = state {
            state.add_static_lines(1);
        }
    });
    choice.value
}

/// The inline arrow menu. Keys come from `next_key` so tests can script
/// them; the real terminal reader is [`read_key`].
pub fn arrow_choice(
    out: &mut dyn Write,
    next_key: &mut dyn FnMut() -> Key,
    ticker: Option<&NewsTicker>,
    title: &str,
    options: &[Choice],
    default: &str,
) -> Result<&'static str, Cancelled> {
    let mut selected = options.iter().position(|c| c.value == default).unwrap_or(0);
    let line_count = options.len() + 1;
    let mut first_draw = true;
    loop {
        with_draw_lock(ticker, |state| {
            let mut text = String::new();
            if !first_draw {
                text.push_str(&format!("\x1b[{line_count}A"));
            }
            text.push_str(&format!("{CLEAR_LINE}{}\n", prompt_line(title)));
            for (index, choice) in options.iter().enumerate() {
                text.push_str(&format!(
                    "{CLEAR_LINE}{}\n",
                    menu_line(choice.label, index == selected)
                ));
            }
            let _ = out.write_all(text.as_bytes());
            let _ = out.flush();
            if let Some(state) = state {
                state.set_active_prompt_lines(line_count);
            }
        });
        first_draw = false;
        match next_key() {
            Key::Up => selected = (selected + options.len() - 1) % options.len(),
            Key::Down => selected = (selected + 1) % options.len(),
            Key::Enter => return Ok(accept(out, ticker, title, &options[selected], line_count)),
            Key::Cancel => {
                clear_menu(out, ticker, line_count);
                return Err(Cancelled::ByUser);
            }
            Key::Digit(n) if (1..=options.len()).contains(&n) => {
                return Ok(accept(out, ticker, title, &options[n - 1], line_count));
            }
            Key::Digit(_) | Key::Other => {}
        }
    }
}

/// Raw mode for the duration of one key read, restored on every exit path.
struct RawMode(bool);

impl RawMode {
    fn enable() -> RawMode {
        RawMode(crossterm::terminal::enable_raw_mode().is_ok())
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        if self.0 {
            let _ = crossterm::terminal::disable_raw_mode();
        }
    }
}

/// A menu key from a terminal key event. Raw mode is on only while a key is
/// awaited, so a Return typed before that arrives as `\n`, which raw mode
/// reports as Ctrl-J: it is Enter, and `j`/`k`/`q`/digits count only
/// without Ctrl or Alt.
pub fn map_key(key: KeyEvent) -> Key {
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    let plain = !key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
    match key.code {
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Char('k') if plain => Key::Up,
        KeyCode::Char('j') if plain => Key::Down,
        KeyCode::Enter => Key::Enter,
        KeyCode::Char('j') if control => Key::Enter,
        KeyCode::Esc => Key::Cancel,
        KeyCode::Char('q') if plain => Key::Cancel,
        KeyCode::Char('c') if control => Key::Cancel,
        KeyCode::Char(c) if plain && c.is_ascii_digit() => Key::Digit(c as usize - '0' as usize),
        _ => Key::Other,
    }
}

/// One key from the terminal (raw mode only while waiting, like readchar).
pub fn read_key(interrupt: &StopSignals) -> Key {
    let _raw = RawMode::enable();
    loop {
        if interrupt.exit_code().is_some() {
            return Key::Cancel;
        }
        match event::poll(Duration::from_millis(50)) {
            Ok(false) => continue,
            Err(_) => return Key::Cancel,
            Ok(true) => {}
        }
        match event::read() {
            Ok(Event::Key(key)) if key.kind != KeyEventKind::Release => return map_key(key),
            Ok(_) => continue,
            Err(_) => return Key::Cancel,
        }
    }
}

/// A key while an answer is typed at the numbered prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKey {
    Char(char),
    Backspace,
    Enter,
    Cancel,
    Other,
}

/// Assemble one typed line from keys, echoing as a cooked terminal would.
/// Enter submits; Ctrl-C, Esc and Ctrl-D on an empty line cancel. `next_key`
/// sees the line typed so far.
pub fn assemble_line(
    out: &mut dyn Write,
    next_key: &mut dyn FnMut(&str) -> LineKey,
) -> Result<String, Cancelled> {
    let mut line = String::new();
    loop {
        match next_key(&line) {
            LineKey::Char(c) => {
                line.push(c);
                let _ = write!(out, "{c}");
            }
            LineKey::Backspace => {
                if line.pop().is_some() {
                    let _ = out.write_all(b"\x08 \x08");
                }
            }
            LineKey::Enter => {
                let _ = out.write_all(b"\r\n");
                let _ = out.flush();
                return Ok(line);
            }
            LineKey::Cancel => {
                let _ = out.write_all(b"\r\n");
                let _ = out.flush();
                return Err(Cancelled::ByUser);
            }
            LineKey::Other => {}
        }
        let _ = out.flush();
    }
}

/// A key of the typed answer; Ctrl-J (a `\n` typed before raw mode) is
/// Enter, as in [`map_key`].
pub fn map_line_key(key: KeyEvent, line_is_empty: bool) -> LineKey {
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Char('c') if control => LineKey::Cancel,
        KeyCode::Char('d') if control && line_is_empty => LineKey::Cancel,
        KeyCode::Char('j') if control => LineKey::Enter,
        KeyCode::Char(c) if !control => LineKey::Char(c),
        KeyCode::Enter => LineKey::Enter,
        KeyCode::Backspace => LineKey::Backspace,
        KeyCode::Esc => LineKey::Cancel,
        _ => LineKey::Other,
    }
}

fn read_line_key(line_is_empty: bool, interrupt: &StopSignals) -> LineKey {
    loop {
        if interrupt.exit_code().is_some() {
            return LineKey::Cancel;
        }
        match event::poll(Duration::from_millis(50)) {
            Ok(false) => continue,
            Err(_) => return LineKey::Cancel,
            Ok(true) => {}
        }
        match event::read() {
            Ok(Event::Key(key)) if key.kind != KeyEventKind::Release => {
                return map_line_key(key, line_is_empty);
            }
            Ok(_) => continue,
            Err(_) => return LineKey::Cancel,
        }
    }
}

/// The answer typed at a terminal, read under raw mode so Ctrl-C cancels
/// the prompt (and init prints `^C Init aborted.`) instead of killing the
/// process.
pub fn read_terminal_line(
    out: &mut dyn Write,
    interrupt: &StopSignals,
) -> Result<String, Cancelled> {
    let _raw = RawMode::enable();
    assemble_line(out, &mut |line| read_line_key(line.is_empty(), interrupt))
}

/// One line of piped stdin; EOF cancels as [`Cancelled::InputClosed`].
#[cfg(any(not(unix), test))]
pub fn read_piped_line(input: &mut dyn BufRead) -> Result<String, Cancelled> {
    let mut line = String::new();
    match input.read_line(&mut line) {
        Ok(0) | Err(_) => Err(Cancelled::InputClosed),
        Ok(_) => Ok(line),
    }
}

/// Read a pipe without holding a blocking buffered read across a signal.
/// One byte at a time leaves subsequent answers in the OS pipe, so each
/// prompt can poll the same stdin without losing buffered input.
#[cfg(unix)]
fn read_stdin_line(interrupt: &StopSignals) -> Result<String, Cancelled> {
    let mut bytes = Vec::new();
    loop {
        if interrupt.exit_code().is_some() {
            return Err(Cancelled::ByUser);
        }
        let mut fd = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };
        // stdin is borrowed, never closed; only this prompt reads it. poll
        // bounds the wait, and read consumes at most one available byte.
        let ready = unsafe { libc::poll(&mut fd, 1, 50) };
        if ready == 0 {
            continue;
        }
        if ready < 0 {
            if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(Cancelled::InputClosed);
        }
        let mut byte = 0u8;
        match unsafe { libc::read(libc::STDIN_FILENO, (&mut byte as *mut u8).cast(), 1) } {
            1 => {
                bytes.push(byte);
                if byte != b'\n' {
                    continue;
                }
            }
            0 if !bytes.is_empty() => {}
            _ => return Err(Cancelled::InputClosed),
        }
        return String::from_utf8(bytes).map_err(|_| Cancelled::InputClosed);
    }
}

#[cfg(not(unix))]
fn read_stdin_line(_interrupt: &StopSignals) -> Result<String, Cancelled> {
    read_piped_line(&mut io::stdin().lock())
}

/// The numbered prompt (rich `Prompt.ask`) for piped or `CI` runs. Accepts
/// a 1-based index or an option value; empty input takes the default;
/// `read_answer` yields one typed line or cancels.
pub fn numbered_choice(
    ui: &Ui,
    read_answer: &mut dyn FnMut() -> Result<String, Cancelled>,
    title: &str,
    options: &[Choice],
    default: &str,
    title_style: Style,
) -> Result<&'static str, Cancelled> {
    let default_index = options
        .iter()
        .position(|c| c.value == default)
        .map_or(1, |i| i + 1);
    ui.blank();
    ui.print(&format!("  {}", ui.styled(title_style, title)));
    for (index, choice) in options.iter().enumerate() {
        let marker = if index + 1 == default_index {
            format!(" {}", ui.styled(DIM, "(default)"))
        } else {
            String::new()
        };
        ui.print(&format!(
            "  {}. {}{marker}",
            index + 1,
            ui.styled(BOLD, choice.label)
        ));
        ui.print(&format!("     {}", ui.styled(DIM, choice.description)));
    }
    loop {
        {
            let mut out = io::stdout().lock();
            let _ = write!(
                out,
                "  Select {}: ",
                ui.styled(BOLD_CYAN, &format!("({default_index})"))
            );
            let _ = out.flush();
        }
        let answer = read_answer()?;
        let answer = answer.trim();
        if answer.is_empty() {
            return Ok(options[default_index - 1].value);
        }
        if let Ok(n) = answer.parse::<usize>() {
            if (1..=options.len()).contains(&n) {
                return Ok(options[n - 1].value);
            }
        } else if let Some(choice) = options.iter().find(|c| c.value == answer) {
            return Ok(choice.value);
        }
        ui.print_styled(RED, "Please select one of the available options");
    }
}

/// Ask through the arrow menu when the terminal allows it, else numbered:
/// typed at the terminal under raw mode, or one line per answer from a pipe.
#[allow(clippy::too_many_arguments)]
pub fn ask_choice(
    ui: &Ui,
    ticker: Option<&NewsTicker>,
    arrow: bool,
    interrupt: &StopSignals,
    title: &str,
    options: &[Choice],
    default: &str,
    title_style: Style,
) -> Result<&'static str, Cancelled> {
    if arrow {
        arrow_choice(
            &mut io::stdout(),
            &mut || read_key(interrupt),
            ticker,
            title,
            options,
            default,
        )
    } else if io::stdin().is_terminal() {
        let mut read = || read_terminal_line(&mut io::stdout(), interrupt);
        numbered_choice(ui, &mut read, title, options, default, title_style)
    } else {
        let mut read = || read_stdin_line(interrupt);
        numbered_choice(ui, &mut read, title, options, default, title_style)
    }
}

#[cfg(test)]
#[path = "select_tests.rs"]
mod tests;
