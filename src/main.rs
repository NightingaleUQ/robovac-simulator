//extern crate ncurses;

use std::io::{
    stdout, Write
};
use rand::Rng;

use crossterm::{
    queue, event,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal, cursor,
    ExecutableCommand, Result,
};

struct Room {
    xsize: u16,
    ysize: u16,
    dust : Vec<(u16, u16)>,
}

impl Room {
    fn new(xsize: u16, ysize: u16) -> Room {
        let mut rng = rand::thread_rng();
        let mut dust: Vec<(u16, u16)> = Vec::new();
        for _ in 0..((xsize * ysize) / 10) {
            let x = rng.gen_range(0..xsize);
            let y = rng.gen_range(0..ysize);
            dust.push((x, y));
        }
        Room{xsize, ysize, dust}
    }
    fn draw(&self) -> Result<()> {
        let mut stdout = stdout();
        queue!(stdout, terminal::Clear(terminal::ClearType::All))?;

        for (x, y) in &self.dust {
            queue!(stdout,
                   cursor::MoveTo((2 * x) + 1, y + 1),
                   SetForegroundColor(Color::Red),
                   Print("<>"),
                   //Print("\u{26AA}"),
                   ResetColor)?;
        }

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
               cursor::MoveTo(2, 0),
               SetForegroundColor(Color::Green),
               Print("Robot Vacuum Simulator!"),
               ResetColor)?;

        stdout.flush()?;
        Ok(())
    }
}

fn main() {
    let (w, h) = terminal::size().unwrap(); 
    let (w, h) = ((w - 2) / 2, h - 8);
    
    let room = Room::new(w, h);
    _ = room.draw();

    loop {
        //let c = ncurses::getch();
    }
    
}
