use rand::{
    Rng, rngs::ThreadRng,
};

use crossterm::{
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal, cursor,
    queue, Result,
};

use std::io::{
    Write, stdout,
};

pub enum Action {
    FORWARD,
    L,
    R,
    SUCK,
}

pub struct Room {
    /* Game board, stored in an array (xsize * ysize) in length 
     * >0: Amount of dirt on space
     *  0: Empty
     * -1: Charging pad
     * -2: Obstacle
     * -3: Hazard */
    board: Vec<i32>,
    xsize: i32,
    ysize: i32,

    /* Robot position and heading:
     * Robots occupy a 4x4 space. Origin is at 1, 1. */
    x: i32,
    y: i32,
    dirn: i32, /* 0: UP, 1: RIGHT, 2: DOWN, 3: LEFT */
}

impl Room {
    pub fn new(xsize: i32, ysize: i32) -> Room {
        let mut rng = rand::thread_rng();
        let board: Vec<i32> = vec![0; (xsize * ysize) as usize];
        let mut room = Room{xsize, ysize, board, x: 1, y: ysize - 3, dirn: 0};
        /* Charging station */
        for x in 0..4 {
            for y in 0..4 {
                room.board[(((ysize - 4 + y) * xsize) + x) as usize] = -1;
            }
        }
        /* Place obstacles and hazards */
        for _ in 0..((xsize * ysize) / 200) {
            room.place_obstacle_or_hazard(rng.gen_range(4..12), -2, &mut rng);
            room.place_obstacle_or_hazard(rng.gen_range(4..12), -3, &mut rng);
        }
        /* Distribute dirt */
        for _ in 0..((xsize * ysize) / 10) {
            room.place_dirt(&mut rng);
        }
        room
    }
    fn place_dirt(&mut self, rng: &mut ThreadRng) {
        let x = rng.gen_range(0..self.xsize);
        let y = rng.gen_range(0..self.ysize);
        let i: usize = (y * self.xsize + x) as usize;
        if self.board[i] >= 0 {
            self.board[i] += 1;
        }
    }
    fn place_obstacle_or_hazard(&mut self, size: i32, tiletype: i32, rng: &mut ThreadRng) {
        /* We're going to randomly generate a shape by growing it from the
         * middle. We begin with a core and keep track of its bounds. */
        let xseed = rng.gen_range(0..self.xsize);
        let yseed = rng.gen_range(0..self.ysize);
        let mut xmin: i32 = xseed;
        let mut xmax: i32 = xseed;
        let mut ymin: i32 = yseed;
        let mut ymax: i32 = yseed;
        let i = (yseed * self.xsize + xseed) as usize;
        if self.board[i] == 0 {
            self.board[i] = tiletype;
        } else {
            return;
        }

        /* And then we add adjacent cells, sliding them out in one of four directions. */
        for _ in 0..size {
            let dirn = rng.gen_range(0..4);
            let dx: i32;
            let dy: i32;
            let startx: i32;
            let starty: i32;
            if dirn & 0x1 == 0 {
                /* Even: Up or down */
                dx = 0;
                dy = if dirn == 0 {-1} else {1};
                startx = rng.gen_range(xmin..=xmax);
                starty = yseed;
            } else {
                /* Odd: Right or left */
                dx = if dirn == 1 {1} else {-1};
                dy = 0;
                startx = xseed;
                starty = rng.gen_range(ymin..=ymax);
            }
            let mut x = startx;
            let mut y = starty;
            /* Slide outwards */
            loop {
                x += dx;
                y += dy;
                if x < 0 || x >= self.xsize || y < 0 || y >= self.ysize {
                    /* Out of bounds, don't grow here */
                    break;
                }
                if x < 8 && y > self.ysize - 8 {
                    /* Too close to charging station */
                    break;
                }
                let i = (y * self.xsize + x) as usize;
                if self.board[i] == 0 {
                    self.board[i] = tiletype;
                    xmin = if x < xmin {x} else {xmin};
                    xmax = if x > xmax {x} else {xmax};
                    ymin = if y < ymin {y} else {ymin};
                    ymax = if y > ymax {y} else {ymax};
                    break;
                }
            }
        }
    }
    pub fn perform_action(&mut self, a: Action) {
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
    pub fn draw(&self) -> Result<()> {
        let mut stdout = stdout();
        queue!(stdout, terminal::Clear(terminal::ClearType::All))?;

        /* Draw room features */
        for x in 0..self.xsize {
            for y in 0..self.ysize {
                let i: usize = (y * self.xsize + x) as usize;
                let (xscr, yscr) = ((2 * x + 1) as u16, (y + 1) as u16);
                queue!(stdout, cursor::MoveTo(xscr, yscr))?;
                match &self.board[i] {
                    -3 => {
                        queue!(stdout, SetForegroundColor(Color::Red), Print("!!"))?;
                    }
                    -2 => {
                        queue!(stdout, SetForegroundColor(Color::Yellow), Print("XX"))?;
                    }
                    -1 => {
                        queue!(stdout, SetForegroundColor(Color::Cyan), Print("OO"))?;
                    }
                    0 => {},
                    d => {
                        match d {
                            1 => { queue!(stdout, SetForegroundColor(Color::AnsiValue(243)))? },
                            2 => { queue!(stdout, SetForegroundColor(Color::AnsiValue(247)))? },
                            3 => { queue!(stdout, SetForegroundColor(Color::AnsiValue(251)))? },
                            _ => { queue!(stdout, SetForegroundColor(Color::AnsiValue(255)))? },
                        }
                        queue!(stdout, Print("<>"))?;
                    }
                }
                queue!(stdout, ResetColor)?;
            }
        }

        /* Draw the robot */
        let (xorig, yorig): (u16, u16) = ((2 * self.x + 1) as u16, (self.y + 1) as u16);
        //queue!(stdout, SetForegroundColor(Color::Black), SetBackgroundColor(Color::Grey))?;
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
        queue!(stdout, cursor::MoveTo(0, (self.ysize + 1) as u16), Print("\u{255A}"))?;
        for _ in 0..self.xsize {
            queue!(stdout, Print("\u{2550}\u{2550}"))?;
        }
        queue!(stdout, Print("\u{255D}"))?;
        for i in 0..(self.ysize as u16) {
            queue!(stdout,
                   cursor::MoveTo(0, i + 1),
                   Print("\u{2551}"),
                   cursor::MoveTo((self.xsize * 2 + 1) as u16, (i + 1) as u16),
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


