use crossterm::{cursor, execute, style::Color, terminal};
use std::io::{self, BufWriter, Write};
use std::{net::TcpStream, sync::mpsc, thread};

mod error;
mod frame;
mod rw;
mod console;

use frame::{Frame, Frame::*};
use rw::{FrameReader, WriteFrame};
use error::ReadError;
use console::Console;

const SERVER: Color = Color::Yellow;
const ERROR: Color = Color::Red;
const CHAT: Color = Color::White;

fn main() -> Result<(), ReadError> {
    let mut buf = String::with_capacity(16);
    let mut name;
    while {
        println!("What is your name?");
        console::read_line(&mut buf)?;
        name = buf.trim();
        name.is_empty() || !name.is_ascii()
    } {}

    let stream = TcpStream::connect("127.0.0.1:6379")?;

    let mut reader = FrameReader::new(stream.try_clone()?);
    let mut writer = BufWriter::new(stream);

    execute!(
        io::stdout(),
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 17),
    )?;

    writer.write_frame(&Slice(&[Simple("JOIN"), Simple(name)]))?;
    writer.flush()?;

    let mut console = Console::new();

    let (tx, rx) = mpsc::sync_channel(128);

    let ctrlc_tx = tx.clone();
    ctrlc::set_handler(move || {
        ctrlc_tx.send(Event::Disconnect).unwrap();
    })
    .expect("Failed to set Ctrl-C handler");

    // thread waiting for server frames
    let server_tx = tx.clone();
    thread::spawn(move || loop {
        match reader.read_frame() {
            Ok(guard) => {
                if server_tx.send(Event::peer_frame(&guard.frame()).unwrap()).is_err() {
                    return;
                }
            }
            Err(e) => {
                if !e.is_exhausted() {
                    println!("error occurred while reading frame: {e:?}");
                }
                let _ = server_tx.send(Event::Disconnect);
                return;
            }
        }
    });

    // thread waiting for user inputs
    thread::spawn(move || -> io::Result<()> {
        let mut buf = String::with_capacity(64);
        loop {
            console::read_line(&mut buf)?;

            if tx
                .send(Event::User {
                    message: buf.clone(),
                })
                .is_err()
            {
                return Ok(());
            }

            // Put cursor back at start of line and clear it
            execute!(
                io::stdout(),
                cursor::MoveTo(0, 17),
                terminal::Clear(terminal::ClearType::FromCursorDown),
            )?;

            // Reset the buffer for the next message
            buf.clear();
        }
    });

    // Main thread is the dispatcher
    for message in rx.iter() {
        match message {
            Event::User { message } => {
                writer.write_frame(&Slice(&[Simple("MSG"), Simple(&message)]))?;
                writer.flush()?;
            }
            Event::Peer { message } => {
                console.write_line(CHAT, message);
            }
            Event::Join { username } => {
                console.write_line(SERVER, format_args!("{username} joined"));
            }
            Event::Leave { username } => {
                console.write_line(SERVER, format_args!("{username} left"));
            }
            Event::Error(e) => {
                console.write_line(ERROR, e);
            }
            Event::Disconnect => {
                return Ok(());
            }
        }
        console.flush()?;
    }

    Ok(())
}

enum Event {
    // User sent a message
    User { message: String },
    // Peer sent a message
    Peer { message: String },
    // Someone else joined
    Join { username: String },
    // Someone else left
    Leave { username: String },
    // Error occurred
    Error(String),
    // User disconnecting
    Disconnect,
}

impl Event {
    fn peer_frame(frame: &Frame<'_>) -> Option<Self> {
        let mut frames = frame.as_array()?.iter();

        let first = frames.next()?;
        if let Error(e) = first {
            return Some(Self::Error(e.to_string()));
        }

        match first.as_simple()? {
            "MSG" => {
                let message = match frames.next()? {
                    Array(array) => {
                        let mut message = String::with_capacity(32);
                        for frame in array.iter() {
                            match frame {
                                Simple(simple) => message += simple,
                                Integer(int) => message += itoa::Buffer::new().format(*int),
                                _ => continue,
                            }
                        }
                        message
                    }
                    Simple(simple) => simple.to_string(),
                    _ => return None,
                };

                Some(Event::Peer { message })
            }
            "JOIN" => {
                let username = frames.next()?.as_simple()?;

                Some(Event::Join {
                    username: username.to_string(),
                })
            }
            "LEAVE" => {
                let username = frames.next()?.as_simple()?;

                Some(Event::Leave {
                    username: username.to_string(),
                })
            }
            _ => None,
        }
    }
}
