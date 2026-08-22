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

/// Writes bytes straight to the terminal. Used for the OSC 52 clipboard write,
/// which is built in `birch_core::clipboard` because choosing a transport needs
/// no terminal — but can only be *delivered* from here, which owns one.
pub fn write_raw(terminal: &mut Term, bytes: &[u8]) {
    let writer = terminal.backend_mut();
    let _ = writer.write_all(bytes);
    let _ = writer.flush();
}

/// Puts a visible caret at a screen position. Only an inline edit does this:
/// the tree draws its own cursor as a highlight, and a blinking caret with no
/// text field under it would be a lie.
pub fn show_caret(terminal: &mut Term, x: u16, y: u16) {
    write_raw(
        terminal,
        format!("\x1b[{};{}H\x1b[?25h", y + 1, x + 1).as_bytes(),
    );
}

pub fn hide_caret(terminal: &mut Term) {
    write_raw(terminal, b"\x1b[?25l");
}

/// Takes the terminal. `EnableMouseCapture` requests modes 1000, 1002, 1003,
/// 1015 and 1006 as one set, and that set stays as it is for the whole session:
/// **nothing turns a mouse mode off while birch runs.** Resetting one — 1003,
/// say, to stop paying for motion the tree does not use — drops reporting below
/// the level the terminal was told to use, and the terminal takes the pointer
/// back. Declining a mode means not asking for it here, in the initial set.
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
