mod game;

use std::io::{
    stdout,
};

use crossterm::{
    event, event::Event,
    terminal, cursor,
    ExecutableCommand,
};

use game::{
    Room, Action,
};

fn main() {
    terminal::enable_raw_mode().expect("Failed to enable RAW mode.");
    _ = stdout().execute(cursor::Hide);

    let (w, h) = terminal::size().unwrap(); 
    let (w, h): (i32, i32) = (((w - 2) / 2) as i32, (h - 8) as i32);
    
    let mut room = Room::new(w, h);
    _ = room.draw(true);

    loop {
        let read = event::read().unwrap();
        match read {
            Event::Key(event::KeyEvent{code: event::KeyCode::Up, ..}) => {
                room.perform_action(Action::FORWARD);
            }
            Event::Key(event::KeyEvent{code: event::KeyCode::Left, ..}) => {
                room.perform_action(Action::L);
            }
            Event::Key(event::KeyEvent{code: event::KeyCode::Right, ..}) => {
                room.perform_action(Action::R);
            }
            Event::Key(event::KeyEvent{code: event::KeyCode::Char(' '), ..}) => {
                room.perform_action(Action::SUCK);
            }
            Event::Key(event::KeyEvent{code: event::KeyCode::Esc, ..}) => {
                break;
            }
            _ => { continue; }
        }
        _ = room.draw(false);
    }
    _ = stdout().execute(terminal::Clear(terminal::ClearType::All));
}
