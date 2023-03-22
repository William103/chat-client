use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::style::{Color, Stylize};
use crossterm::{cursor, style, terminal, QueueableCommand};
use std::fmt::Write as _;
use std::io::{self, stdout, Write};
use std::{array, fmt};

pub fn read_line(buf: &mut String) -> io::Result<()> {
    loop {
        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Enter => return Ok(()),
                KeyCode::Char(c) => buf.push(c),
                _ => {}
            }
        }
    }
}

pub struct Console {
    lines: Ring<(String, Color), 16>,
}

impl Console {
    pub fn new() -> Self {
        Console {
            lines: Ring::new(array::from_fn(|_| {
                (String::with_capacity(64), Color::White)
            })),
        }
    }

    pub fn write_line(&mut self, color: Color, args: impl fmt::Display) {
        let (next_line, next_color) = self.lines.next_mut();
        next_line.clear();
        write!(next_line, "{args}").expect("writing to a string shouldn't fail");
        *next_color = color;
    }

    pub fn flush(&self) -> io::Result<()> {
        let mut stdout = stdout();
        stdout
            .queue(cursor::SavePosition)?
            .queue(cursor::MoveToRow(16))?
            .queue(terminal::Clear(terminal::ClearType::FromCursorUp))?;

        for (row, (line, color)) in self.lines.iter().enumerate() {
            stdout
                .queue(cursor::MoveTo(0, row as u16))?
                .queue(style::PrintStyledContent(line.as_str().with(*color)))?;
        }

        stdout.queue(cursor::RestorePosition)?.flush()
    }
}

/// Fixed-size ring buffer
struct Ring<T, const N: usize> {
    buf: [T; N],
    head: usize,
}

impl<T, const N: usize> Ring<T, N> {
    fn new(buf: [T; N]) -> Self {
        Ring { buf, head: 0 }
    }

    fn next_mut(&mut self) -> &mut T {
        let res = &mut self.buf[self.head];
        self.head += 1;
        if self.head == N {
            self.head = 0;
        }
        res
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        let (around, head) = self.buf.split_at(self.head);
        head.iter().chain(around.iter())
    }
}
