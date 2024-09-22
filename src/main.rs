use clap::{arg, command};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    style::{Color, Print, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
    ExecutableCommand,
};
use std::process::{Command, ExitStatus, Output};
use std::{env, io};
use std::{error::Error, fmt, io::Write, process};

pub const COMMIT_TYPES: &'static [&str] = &[
    "feat", "fix", "docs", "style", "refactor", "perf", "test", "ci", "chore",
];

#[derive(Debug)]
enum GczError {
    Io(io::Error),
    UserInterrupt,
}

impl fmt::Display for GczError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GczError::Io(err) => write!(f, "IO error: {}", err),
            GczError::UserInterrupt => write!(f, "Interrupted by user"),
        }
    }
}

impl Error for GczError {}

impl From<io::Error> for GczError {
    fn from(err: io::Error) -> Self {
        GczError::Io(err)
    }
}

fn graceful_shutdown(stdout: &mut io::Stdout) -> io::Result<()> {
    disable_raw_mode().and_then(|_| execute!(stdout, cursor::Show))
}

fn main() {
    let _ = command!()
        .arg(arg!(-e --emoji "WIP: add emoji to commit template").required(false))
        .get_matches();

    let stdout = &mut io::stdout();

    match gcz(stdout) {
        Ok(_) => {}
        Err(GczError::UserInterrupt) => {
            graceful_shutdown(stdout).expect("Failed to shutdown");
            process::exit(1);
        }
        Err(err) => {
            eprintln!("Error: {}", err);
            graceful_shutdown(stdout).expect("Failed to shutdown");
            process::exit(1);
        }
    }
}

fn gcz(stdout: &mut io::Stdout) -> Result<(), GczError> {
    if !is_inside_git_dir()?.stdout.starts_with(b"true") {
        println!("Not a git repository");
        return Ok(());
    }

    if exist_stages_changes()?.success() {
        println!("No staged changes");
        return Ok(());
    }

    let selected_type = select_commit_type(stdout)?;
    let message = input_commit_message(stdout, &selected_type)?;

    let status = Command::new("git")
        .args(&["commit", "-m", &message])
        .status()?;

    if status.success() {
        println!("Commit successful: {}", message);
    } else {
        println!("Commit failed");
    }

    Ok(())
}

fn is_inside_git_dir() -> Result<Output, GczError> {
    Command::new("git")
        .args(&["rev-parse", "--is-inside-work-tree"])
        .output()
        .map_err(GczError::from)
}

fn exist_stages_changes() -> Result<ExitStatus, GczError> {
    Command::new("git")
        .args(&["diff", "--cached", "--exit-code"])
        .status()
        .map_err(GczError::from)
}

fn select_commit_type(stdout: &mut io::Stdout) -> Result<String, GczError> {
    enable_raw_mode()
        .map_err(GczError::from)
        .and_then(|_| execute!(stdout, cursor::Hide, Clear(ClearType::All)).map_err(GczError::from))
        .and_then(|_| handle_commit_type(stdout))
        .and_then(|input| finalize(input, stdout))
}

fn handle_commit_type(stdout: &mut io::Stdout) -> Result<String, GczError> {
    let mut selected_index = 0;
    let mut input = String::new();
    let mut is_selected = false;

    loop {
        stdout.execute(Clear(ClearType::All))?;

        if is_selected {
            execute!(
                stdout,
                cursor::MoveTo(0, 0),
                Print("Selected commit type: "),
                SetForegroundColor(Color::Cyan),
                Print(&input),
                SetForegroundColor(Color::Reset),
                cursor::MoveToNextLine(1)
            )?;
            break Ok(input);
        }

        execute!(
            stdout,
            cursor::MoveTo(0, 0),
            Print(format!("Select a commit type: {}", &input)),
            cursor::MoveToNextLine(1)
        )?;

        let filtered_types: Vec<(usize, &'static str)> = filter_type_by_input(&input);

        for (i, &(_, commit_type)) in filtered_types.iter().enumerate() {
            if i == selected_index {
                execute!(
                    stdout,
                    SetForegroundColor(Color::Green),
                    Print(format!("❯ {}", commit_type)),
                    SetForegroundColor(Color::Reset),
                    cursor::MoveToNextLine(1),
                )?;
            } else {
                execute!(
                    stdout,
                    Print(format!("  {}", commit_type)),
                    cursor::MoveToNextLine(1)
                )?;
            }
        }
        stdout.flush()?;

        if let Event::Key(key_event) = event::read()? {
            match (key_event.code, key_event.modifiers) {
                (KeyCode::Char('c'), KeyModifiers::CONTROL)
                | (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                    return Err(GczError::UserInterrupt);
                }
                (KeyCode::Up, _) => {
                    if selected_index > 0 {
                        selected_index -= 1
                    } else {
                        selected_index = filtered_types.len() - 1
                    }
                }
                (KeyCode::Down, _) => {
                    if selected_index < filtered_types.len() - 1 {
                        selected_index += 1
                    } else {
                        selected_index = 0
                    }
                }

                (KeyCode::Enter, _) => {
                    if !filtered_types.is_empty() {
                        input = filtered_types[selected_index].1.to_string();
                        is_selected = true;
                    }
                }
                (KeyCode::Char(c), _) => {
                    input.push(c);
                    selected_index = 0
                }
                (KeyCode::Backspace, _) => {
                    input.pop();
                    selected_index = 0;
                }
                (KeyCode::Esc, _) => {
                    input.clear();
                    selected_index = 0;
                }
                _ => continue,
            }
        }
    }
}

fn filter_type_by_input(input: &str) -> Vec<(usize, &'static str)> {
    COMMIT_TYPES
        .iter()
        .enumerate()
        .filter(|(_, &t)| t.to_lowercase().contains(&input.to_lowercase()))
        .map(|(i, &t)| (i, t))
        .collect()
}

fn finalize(input: String, stdout: &mut io::Stdout) -> Result<String, GczError> {
    disable_raw_mode()?;
    execute!(stdout, cursor::Show, cursor::MoveToNextLine(1))?;
    Ok(input)
}

fn input_commit_message(stdout: &mut io::Stdout, commit_type: &str) -> Result<String, GczError> {
    let mut message = format!("{}: ", commit_type);
    let mut cursor_pos = message.len();

    enable_raw_mode()?;
    loop {
        execute!(
            stdout,
            Clear(ClearType::CurrentLine),
            cursor::MoveToColumn(0),
            Print(&message),
            cursor::MoveToColumn(cursor_pos as u16)
        )
        .and_then(|_| stdout.flush())?;

        if let Event::Key(key_event) = event::read()? {
            match (key_event.code, key_event.modifiers) {
                (KeyCode::Char('c'), KeyModifiers::CONTROL)
                | (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                    disable_raw_mode()?;
                    return Err(GczError::UserInterrupt);
                }
                (KeyCode::Enter, _) => {
                    disable_raw_mode()?;
                    execute!(stdout, cursor::MoveToNextLine(2))?;
                    return Ok(message);
                }
                (KeyCode::Char(c), _) => {
                    message.insert(cursor_pos, c);
                    cursor_pos += 1;
                }
                (KeyCode::Backspace, _) if cursor_pos > 0 => {
                    message.remove(cursor_pos - 1);
                    cursor_pos -= 1;
                }
                (KeyCode::Delete, _) if cursor_pos < message.len() => {
                    message.remove(cursor_pos);
                }
                (KeyCode::Left, _) if cursor_pos > 0 => cursor_pos -= 1,
                (KeyCode::Right, _) if cursor_pos < message.len() => cursor_pos += 1,
                (KeyCode::Home, _) => cursor_pos = 0,
                (KeyCode::End, _) => cursor_pos = message.len(),
                _ => continue,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_filter() {
        let input = "f";
        let result = filter_type_by_input(input);
        assert_eq!(
            result,
            vec![(0, "feat"), (1, "fix"), (4, "refactor"), (5, "perf")]
        );
    }

    #[test]
    fn should_finalize_correctly() {
        let input = "feat";
        let mut stdout = io::stdout();
        let result = finalize(input.to_string(), &mut stdout).unwrap();

        assert_eq!(result, "feat");
    }
}
