use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io::{self, stdout, IsTerminal, Read, Stdout, Write};
use std::process::{Command, Stdio};

#[cfg(unix)]
use std::os::fd::AsRawFd;

type Backend = CrosstermBackend<Stdout>;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    StartPoint,
    EndPoint,
    Done,
}

#[derive(Debug, Clone, PartialEq)]
struct Match {
    line: usize,
    start: usize,
    end: usize,
    text: String,
}

#[derive(Debug)]
struct SpotcheckState {
    mode: Mode,
    buffer: Vec<String>,
    input: String,
    matches: Vec<Match>,
    selected_match: usize,
    start_point: Option<(usize, usize)>, // (line, column)
    end_point: Option<(usize, usize)>,
    message: String,
}

impl SpotcheckState {
    fn new(buffer: Vec<String>) -> Self {
        Self {
            mode: Mode::StartPoint,
            buffer,
            input: String::new(),
            matches: Vec::new(),
            selected_match: 0,
            start_point: None,
            end_point: None,
            message: "SPOTCHECK > Type to find start point".to_string(),
        }
    }

    fn search(&mut self) {
        if self.input.is_empty() {
            self.matches.clear();
            self.selected_match = 0;
            return;
        }

        self.matches.clear();

        for (line_idx, line) in self.buffer.iter().enumerate() {
            // Simple substring search for now
            if let Some(start) = line.to_lowercase().find(&self.input.to_lowercase()) {
                let end = start + self.input.len();
                self.matches.push(Match {
                    line: line_idx,
                    start,
                    end,
                    text: line[start..end].to_string(),
                });
            }
        }

        if !self.matches.is_empty() {
            self.selected_match = 0;
        }
    }

    fn confirm_selection(&mut self) {
        if let Some(match_) = self.matches.get(self.selected_match) {
            match self.mode {
                Mode::StartPoint => {
                    self.start_point = Some((match_.line, match_.start));
                    self.mode = Mode::EndPoint;
                    self.input.clear();
                    self.matches.clear();
                    self.message = "ENDPOINT > Type to find end point".to_string();
                }
                Mode::EndPoint => {
                    self.end_point = Some((match_.line, match_.end));
                    self.mode = Mode::Done;
                    self.message = "Confirmed! Press Enter to copy to clipboard".to_string();
                }
                Mode::Done => {}
            }
        }
    }

    fn extract_selection(&self) -> Option<String> {
        let (start_line, start_col) = self.start_point?;
        let (end_line, end_col) = self.end_point?;

        if start_line == end_line {
            let line = self.buffer.get(start_line)?;
            let start = start_col.min(line.len());
            let end = end_col.min(line.len());
            if start < end {
                return Some(line[start..end].to_string());
            }
        } else if start_line < end_line {
            let mut result = String::new();

            // First line (from start_col to end)
            if let Some(line) = self.buffer.get(start_line) {
                let start = start_col.min(line.len());
                result.push_str(&line[start..]);
                result.push('\n');
            }

            // Middle lines (full lines)
            for line in self.buffer.iter().take(end_line).skip(start_line + 1) {
                result.push_str(line);
                result.push('\n');
            }

            // Last line (from start to end_col)
            if let Some(line) = self.buffer.get(end_line) {
                let end = end_col.min(line.len());
                result.push_str(&line[..end]);
            }

            return Some(result);
        }

        None
    }
}

fn strip_ansi(text: &str) -> String {
    let mut clean = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
        } else {
            clean.push(ch);
        }
    }
    clean
}

fn buffer_from_stdin() -> io::Result<Vec<String>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let input = strip_ansi(&input);
    Ok(input.lines().map(str::to_owned).collect())
}

#[cfg(unix)]
struct TtyStdin {
    original_fd: i32,
}

#[cfg(unix)]
impl TtyStdin {
    fn open() -> io::Result<Self> {
        let tty = std::fs::File::open("/dev/tty")?;
        let original_fd = unsafe { libc::dup(0) };
        if original_fd < 0 {
            return Err(io::Error::last_os_error());
        }
        if unsafe { libc::dup2(tty.as_raw_fd(), 0) } < 0 {
            let error = io::Error::last_os_error();
            unsafe {
                libc::close(original_fd);
            }
            return Err(error);
        }
        Ok(Self { original_fd })
    }
}

#[cfg(unix)]
impl Drop for TtyStdin {
    fn drop(&mut self) {
        unsafe {
            libc::dup2(self.original_fd, 0);
            libc::close(self.original_fd);
        }
    }
}

fn run_spotcheck(
    terminal: &mut Terminal<Backend>,
    buffer: Vec<String>,
) -> io::Result<Option<String>> {
    let mut state = SpotcheckState::new(buffer);

    loop {
        terminal.draw(|f| ui(f, &state))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char(c) => {
                    if state.mode != Mode::Done {
                        state.input.push(c);
                        state.search();
                    }
                }
                KeyCode::Backspace => {
                    if state.mode != Mode::Done {
                        state.input.pop();
                        state.search();
                    }
                }
                KeyCode::Up => {
                    if !state.matches.is_empty() && state.selected_match > 0 {
                        state.selected_match -= 1;
                    }
                }
                KeyCode::Down => {
                    if !state.matches.is_empty() {
                        state.selected_match =
                            (state.selected_match + 1).min(state.matches.len() - 1);
                    }
                }
                KeyCode::Enter => {
                    if state.mode == Mode::Done {
                        if let Some(selection) = state.extract_selection() {
                            return Ok(Some(selection));
                        }
                    } else if !state.matches.is_empty() {
                        state.confirm_selection();
                    }
                }
                KeyCode::Esc => {
                    return Ok(None);
                }
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, state: &SpotcheckState) {
    let size = f.area();

    // Layout: input bar at top, content in middle, status at bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(size);

    // Input bar
    let input_text = vec![
        Span::styled(
            &state.message,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(&state.input, Style::default().fg(Color::Green)),
    ];

    let input_paragraph =
        Paragraph::new(Line::from(input_text)).block(Block::default().borders(Borders::ALL));

    f.render_widget(input_paragraph, chunks[0]);

    // Content area with highlighted matches
    let content = render_content(state, chunks[1]);
    f.render_widget(content, chunks[1]);

    // Status bar
    let status_text = if state.mode == Mode::Done {
        format!(
            "Start: {:?}, End: {:?} | [Enter] Copy | [Esc] Cancel",
            state.start_point, state.end_point
        )
    } else {
        format!(
            "{} matches | [↑↓] Navigate | [Enter] Confirm | [Esc] Cancel",
            state.matches.len()
        )
    };

    let status_paragraph =
        Paragraph::new(Line::from(status_text)).block(Block::default().borders(Borders::ALL));

    f.render_widget(status_paragraph, chunks[2]);
}

fn render_content(state: &SpotcheckState, _area: Rect) -> Paragraph<'_> {
    let mut lines = Vec::new();

    for (line_idx, line) in state.buffer.iter().enumerate() {
        let mut spans = Vec::new();

        // Check if this line has any matches
        let line_matches: Vec<_> = state
            .matches
            .iter()
            .filter(|m| m.line == line_idx)
            .collect();

        if line_matches.is_empty() {
            spans.push(Span::raw(line.clone()));
        } else {
            let mut pos = 0;
            for match_ in line_matches.iter() {
                // Text before the match
                if pos < match_.start {
                    spans.push(Span::raw(line[pos..match_.start].to_string()));
                }

                // The match itself
                let is_selected = state.matches.get(state.selected_match) == Some(*match_);
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                };

                spans.push(Span::styled(&match_.text, style));
                pos = match_.end;
            }

            // Text after the last match
            if pos < line.len() {
                spans.push(Span::raw(line[pos..].to_string()));
            }
        }

        lines.push(Line::from(spans));
    }

    Paragraph::new(lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Terminal Buffer"),
    )
}

fn copy_to_clipboard(text: &str) -> io::Result<()> {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if clipboard.set_text(text).is_ok() {
            return Ok(());
        }
    }

    for command in ["wl-copy", "xclip", "xsel"] {
        let mut child = match Command::new(command)
            .args(match command {
                "xclip" => &["-selection", "clipboard"][..],
                "xsel" => &["--clipboard", "--input"][..],
                _ => &[][..],
            })
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(_) => continue,
        };
        if let Some(mut stdin) = child.stdin.take() {
            if stdin.write_all(text.as_bytes()).is_err() {
                continue;
            }
        }
        if child.wait()?.success() {
            return Ok(());
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no usable clipboard backend found",
    ))
}

fn demo_buffer() -> Vec<String> {
    vec![
        "Aug 03 19:42:11 server nginx[4421]: failed to bind port 443".to_string(),
        "Aug 03 19:42:12 server nginx[4421]: retrying".to_string(),
        "Aug 03 19:42:13 server nginx[4421]: successfully bound port 443".to_string(),
        "Aug 03 19:42:14 server nginx[4421]: serving requests".to_string(),
        "".to_string(),
        "Aug 03 19:43:00 server systemd[1]: Started nginx service".to_string(),
        "Aug 03 19:43:01 server kernel: TCP: established socket".to_string(),
    ]
}

fn print_usage() {
    println!("spotcheck {}", env!("CARGO_PKG_VERSION"));
    println!("Precision text extraction for the terminal.");
    println!();
    println!("USAGE:");
    println!("    command | spotcheck");
    println!("    spotcheck --test <start> <end>");
    println!("    spotcheck --help");
    println!("    spotcheck --version");
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 2 && (args[1] == "--help" || args[1] == "-h") {
        print_usage();
        return Ok(());
    }
    if args.len() == 2 && (args[1] == "--version" || args[1] == "-V") {
        println!("spotcheck {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Test mode: --test "start_search" "end_search"
    if args.len() == 4 && args[1] == "--test" {
        let start_search = &args[2];
        let end_search = &args[3];

        let mut state = SpotcheckState::new(demo_buffer());

    if let Some(text) = result {
        println!("Selection: {}", text);
        if copy_to_clipboard(&text).is_err() {
            println!("(Clipboard not available in headless mode)");
        } else {
            println!("Copied to clipboard!");
        }
    } else {
        println!("Selection cancelled.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_common_ansi_sequences() {
        assert_eq!(strip_ansi("\u{1b}[31mred\u{1b}[0m"), "red");
    }

    #[test]
    fn extracts_multiline_selection() {
        let mut state = SpotcheckState::new(vec!["one two".into(), "three four".into()]);
        state.start_point = Some((0, 4));
        state.end_point = Some((1, 5));
        assert_eq!(state.extract_selection().as_deref(), Some("two\nthree"));
    }
}
