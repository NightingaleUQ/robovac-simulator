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

    let (termw, termh) = terminal::size().unwrap();
    let (w, h): (i32, i32) = (((termw - 2) / 2) as i32, (termh - 8) as i32);
    
    let mut room = Room::new(w, h);
    _ = room.draw(true);

    loop {
        let read = event::read().unwrap();
        let a = match read {
            Event::Key(event::KeyEvent{code: event::KeyCode::Up, ..})
                => Action::FORWARD,
            Event::Key(event::KeyEvent{code: event::KeyCode::Down, ..})
                => Action::REVERSE,
            Event::Key(event::KeyEvent{code: event::KeyCode::Left, ..})
                => Action::L,
            Event::Key(event::KeyEvent{code: event::KeyCode::Right, ..})
                => Action::R,
            Event::Key(event::KeyEvent{code: event::KeyCode::Char(' '), ..})
                => Action::SUCK,
            Event::Key(event::KeyEvent{code: event::KeyCode::Esc, ..}) |
            Event::Key(event::KeyEvent{code: event::KeyCode::Char('q'), ..})
                => { break; }
            _
                => { continue; }
        };
        room.perform_action(a);
        _ = room.draw(false);
    }
    _ = stdout().execute(terminal::Clear(terminal::ClearType::All));
    _ = stdout().execute(cursor::Show);
}
