//! Terminal lifecycle: alt screen + raw mode + optional mouse capture, with a
//! panic hook that restores the terminal before the panic message prints.
//! Picker mode renders on stderr so `$(birch --pick)` reads a clean stdout.

use std::io::{self, Write};
use std::panic;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

pub enum TermOut {
    Stdout(io::Stdout),
    Stderr(io::Stderr),
}

impl Write for TermOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            TermOut::Stdout(s) => s.write(buf),
            TermOut::Stderr(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            TermOut::Stdout(s) => s.flush(),
            TermOut::Stderr(s) => s.flush(),
        }
    }
}

fn out(stderr: bool) -> TermOut {
    if stderr {
        TermOut::Stderr(io::stderr())
    } else {
        TermOut::Stdout(io::stdout())
    }
}

pub type Term = Terminal<CrosstermBackend<TermOut>>;

pub fn enter(mouse: bool, stderr: bool) -> io::Result<Term> {
    enable_raw_mode()?;
    let result = (|| {
        let mut stream = out(stderr);
        execute!(stream, EnterAlternateScreen)?;
        if mouse {
            execute!(stream, EnableMouseCapture)?;
        }
        Terminal::new(CrosstermBackend::new(stream))
    })();
    match result {
        Ok(terminal) => {
            let hook = panic::take_hook();
            panic::set_hook(Box::new(move |info| {
                restore(mouse, stderr);
                hook(info);
            }));
            Ok(terminal)
        }
        Err(e) => {
            // Undo whatever partially succeeded; never leave the shell raw.
            restore(mouse, stderr);
            Err(e)
        }
    }
}

pub fn leave(mouse: bool, stderr: bool) {
    restore(mouse, stderr);
    let _ = panic::take_hook(); // drop our hook; the default suffices now
}

/// Best-effort restore; used on normal exit, panic, and child handover.
pub fn restore(mouse: bool, stderr: bool) {
    let mut stream = out(stderr);
    if mouse {
        let _ = execute!(stream, DisableMouseCapture);
    }
    let _ = execute!(stream, LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

/// Re-enter after a child had the terminal (no new panic hook needed).
pub fn reenter(mouse: bool, stderr: bool) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stream = out(stderr);
    execute!(stream, EnterAlternateScreen)?;
    if mouse {
        execute!(stream, EnableMouseCapture)?;
    }
    Ok(())
}
