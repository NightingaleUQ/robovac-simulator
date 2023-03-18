//extern crate ncurses;

use std::io::{
    stdout, Write
};
use rand::Rng;

use crossterm::{
    queue, event, event::Event,
    style::{Color, Print, ResetColor, SetForegroundColor, SetBackgroundColor},
    terminal, cursor,
    ExecutableCommand,
    Result,
};

enum Action {
    FORWARD,
    L,
    R,
    SUCK,
}

struct Room {
    /* Game board, stored in an array (xsize * ysize) in length 
     * >0: Amount of dirt on space
     *  0: Empty
     * -1: Charging pad */
    board: Vec<i32>,
    xsize: u16,
    ysize: u16,

    /* Robot position and heading:
     * Robots occupy a 4x4 space. Origin is at 1, 1. */
    x: u16,
    y: u16,
    dirn: i16, /* 0: UP, 1: RIGHT, 2: DOWN, 3: LEFT */
}

impl Room {
    fn new(xsize: u16, ysize: u16) -> Room {
        let mut rng = rand::thread_rng();
        let mut board: Vec<i32> = vec![0; (xsize * ysize).into()];
        /* Place dirt */
        for _ in 0..((xsize * ysize) / 10) {
            let x = rng.gen_range(0..xsize);
            let y = rng.gen_range(0..ysize);
            let i: usize = (y * xsize + x).into();
            if board[i] >= 0 {
                board[i] += 1;
            }
        }
        /* Charging station */
        for x in 0..4 {
            for y in 0..4 {
                board[(((ysize - 4 + y) * xsize) + x) as usize] = -1;
            }
        }
        Room{xsize, ysize, board, x: 1, y: ysize - 3, dirn: 0}
    }
    fn perform_action(&mut self, a: Action) {
        match a {
            Action::FORWARD => {
                match self.dirn {
                    0 => { self.y -= 1; }
                    1 => { self.x += 1; }
                    2 => { self.y += 1; }
                    3 => { self.x -= 1; }
                    _ => {}
                }
            }
            Action::L => { self.dirn = (self.dirn - 1) & 0x3; }
            Action::R => { self.dirn = (self.dirn + 1) & 0x3; }
            Action::SUCK => {}
        }
    }
    fn draw(&self) -> Result<()> {
        let mut stdout = stdout();
        queue!(stdout, terminal::Clear(terminal::ClearType::All))?;

        /* Draw room features */
        for x in 0..self.xsize {
            for y in 0..self.ysize {
                let i: usize = (y * self.xsize + x).into();
                let (xscr, yscr) = ((2 * x + 1), (y + 1));
                match &self.board[i] {
                    -1 => {
                        queue!(stdout,
                               cursor::MoveTo(xscr, yscr),
                               SetForegroundColor(Color::Cyan),
                               Print("OO"),
                               ResetColor)?;
                    }
                    0 => {},
                    d => {
                        let (x, y): (u16, u16) = (x.try_into().unwrap(), y.try_into().unwrap());
                        queue!(stdout,
                               cursor::MoveTo((2 * x) + 1, y + 1))?;
                        match d {
                            1 => { queue!(stdout, SetForegroundColor(Color::White))? },
                            2 => { queue!(stdout, SetForegroundColor(Color::Magenta))? },
                            _ => { queue!(stdout, SetForegroundColor(Color::Red))? },
                        }
                        queue!(stdout,
                               Print("<>"),
                               ResetColor)?;
                    }
                }
            }
        }

        /* Draw the robot */
        let (xorig, yorig) = ((2 * self.x + 1), (self.y + 1));
        queue!(stdout, SetForegroundColor(Color::Black), SetBackgroundColor(Color::Grey))?;
        match self.dirn {
            0 => {
                /* UP */
                queue!(stdout,
                       cursor::MoveTo(xorig, yorig - 1),
                       Print("####"),
                       cursor::MoveTo(xorig - 2, yorig),
                       Print("|      |"),
                       cursor::MoveTo(xorig - 2, yorig + 1),
                       Print("|      |"),
                       cursor::MoveTo(xorig - 1, yorig + 2),
                       Print("`----'"))?;
            }
            1 => {
                /* RIGHT */
                queue!(stdout,
                       cursor::MoveTo(xorig - 1, yorig - 1),
                       Print(".----"),
                       cursor::MoveTo(xorig - 2, yorig),
                       Print("|      #"),
                       cursor::MoveTo(xorig - 2, yorig + 1),
                       Print("|      #"),
                       cursor::MoveTo(xorig - 1, yorig + 2),
                       Print("`----"))?;
            }
            2 => {
                /* DOWN */
                queue!(stdout,
                       cursor::MoveTo(xorig - 1, yorig - 1),
                       Print(",----."),
                       cursor::MoveTo(xorig - 2, yorig),
                       Print("|      |"),
                       cursor::MoveTo(xorig - 2, yorig + 1),
                       Print("|      |"),
                       cursor::MoveTo(xorig, yorig + 2),
                       Print("####"))?;
            }
            3 => {
                /* LEFT */
                queue!(stdout,
                       cursor::MoveTo(xorig, yorig - 1),
                       Print("----,"),
                       cursor::MoveTo(xorig - 2, yorig),
                       Print("#      |"),
                       cursor::MoveTo(xorig - 2, yorig + 1),
                       Print("#      |"),
                       cursor::MoveTo(xorig, yorig + 2),
                       Print("----'"))?;
            }
            _ => {}
        }
        queue!(stdout, ResetColor)?;

        /* Draw border around room */
        queue!(stdout, cursor::MoveTo(0, 0), Print("\u{2554}"))?;
        for _ in 0..self.xsize {
            queue!(stdout, Print("\u{2550}\u{2550}"))?;
        }
        queue!(stdout, Print("\u{2557}"))?;
        queue!(stdout, cursor::MoveTo(0, self.ysize + 1), Print("\u{255A}"))?;
        for _ in 0..self.xsize {
            queue!(stdout, Print("\u{2550}\u{2550}"))?;
        }
        queue!(stdout, Print("\u{255D}"))?;
        for i in 0..self.ysize {
            queue!(stdout,
                   cursor::MoveTo(0, i + 1),
                   Print("\u{2551}"),
                   cursor::MoveTo(self.xsize * 2 + 1, i + 1),
                   Print("\u{2551}"))?;
        }
        queue!(stdout,
               cursor::MoveTo(1, 0),
               SetForegroundColor(Color::Green),
               Print(" Robot Vacuum Simulator! "),
               ResetColor)?;

        stdout.flush()?;
        Ok(())
    }
}

fn main() {
    terminal::enable_raw_mode().expect("Failed to enable RAW mode.");

    let (w, h) = terminal::size().unwrap(); 
    let (w, h) = ((w - 2) / 2, h - 8);
    
    let mut room = Room::new(w, h);
    _ = room.draw();

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
            _ => {}
        }
        _ = room.draw();
    }
    _ = stdout().execute(terminal::Clear(terminal::ClearType::All));
}
