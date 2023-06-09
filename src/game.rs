use rand::{
    Rng, rngs::ThreadRng,
};

use crossterm::{
    style::{Color, Print, ResetColor, SetForegroundColor, SetBackgroundColor},
    terminal, cursor,
    queue, Result,
};

use std::io::{
    Write, stdout,
};

#[derive(PartialEq)]
pub enum Action {
    FORWARD,
    REVERSE,
    L,
    R,
    SUCK,
}

pub fn i_to_act(i: usize) -> Action {
    match i {
        0 => Action::FORWARD,
        1 => Action::REVERSE,
        2 => Action::L,
        3 => Action::R,
        4 => Action::SUCK,
        _ => Action::FORWARD,
    }
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

    /* Cumulative reward */
    r: f32,
}

pub const SIZE_STATE: usize = 403;
pub const SIZE_ACTION: usize = 5;
pub type RoomVec = [f32; SIZE_STATE as usize];

impl Room {
    pub fn new(xsize: i32, ysize: i32) -> Room {
        let board: Vec<i32> = vec![0; (xsize * ysize) as usize];
        let mut room = Room{xsize, ysize, board, x: 1, y: ysize - 3, dirn: 0, r: 0.0};
        room.generate_level();
        room
    }
    fn generate_level(&mut self) {
        let mut rng = rand::thread_rng();
        self.board.iter_mut().for_each(|x| *x = 0);
        /* Charging station */
        for x in 0..4 {
            for y in 0..4 {
                self.board[(((self.ysize - 4 + y) * self.xsize) + x) as usize] = -1;
            }
        }
        /* Generate room contents */
        for _ in 0..((self.xsize * self.ysize) / 800) {
            self.place_hazard(rng.gen_range(4..12), rng.gen_range(4..12), &mut rng);
        }
        for _ in 0..((self.xsize * self.ysize) / 200) {
            self.place_obstacle(rng.gen_range(4..12), &mut rng);
        }
        for _ in 0..((self.xsize * self.ysize) / 10) {
            self.place_dirt(&mut rng);
        }
    }
    fn place_dirt(&mut self, rng: &mut ThreadRng) {
        let x = rng.gen_range(0..self.xsize);
        let y = rng.gen_range(0..self.ysize);
        let i: usize = (y * self.xsize + x) as usize;
        if self.board[i] >= 0 {
            self.board[i] += 1;
        }
    }
    fn place_obstacle(&mut self, size: i32, rng: &mut ThreadRng) {
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
            self.board[i] = -2;
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
                    self.board[i] = -2;
                    xmin = if x < xmin {x} else {xmin};
                    xmax = if x > xmax {x} else {xmax};
                    ymin = if y < ymin {y} else {ymin};
                    ymax = if y > ymax {y} else {ymax};
                    break;
                } else if self.board[i] != -2 {
                    break;
                }
            }
        }
    }
    fn place_hazard(&mut self, xsize: i32, ysize: i32, rng: &mut ThreadRng) {
        /* Places a rectangualar obstacle of a specified size */
        let xmin = rng.gen_range(0..self.xsize);
        let ymin = rng.gen_range(0..self.ysize);
        let xmax = if xmin + xsize < self.xsize {xmin + xsize} else {self.xsize};
        let ymax = if ymin + ysize < self.ysize {ymin + ysize} else {self.ysize};
        /* Don't get too close to the charging pad */
        if xmin < 8 && ymax > self.ysize - 8 {
            return;
        }
        for x in xmin..xmax {
            for y in ymin..ymax {
                self.board[(y * self.xsize + x) as usize] = -3;
            }
        }
    }
    fn get_suction_range(&self) -> [(i32, i32); 4] {
        /* Returns the four squares immediately in front of the robot,
         * In order from left-to-right relative to the robot's rotation.  */
        match self.dirn {
            0 => [(self.x-1, self.y-1), (self.x, self.y-2), (self.x+1, self.y-2), (self.x+2, self.y-1)],
            1 => [(self.x+2, self.y-1), (self.x+3, self.y), (self.x+3, self.y+1), (self.x+2, self.y+2)],
            2 => [(self.x+2, self.y+2), (self.x+1, self.y+3), (self.x, self.y+3), (self.x-1, self.y+2)],
            3 => [(self.x-1, self.y+2), (self.x-2, self.y+1), (self.x-2, self.y), (self.x-1, self.y-1)],
            _ => [(-1, -1), (-1, -1), (-1, -1), (-1, -1)],
        }
    }
    fn get_occupied_squares(x: i32, y: i32, dirn: i32) -> [(i32, i32); 14] {
        /* Given a (new) vaccuum position, return the squares that are covered by it. */
        match dirn {
            0 => [            (x, y-1), (x+1, y-1),
                  (x-1,  y ), (x,  y ), (x+1,  y ), (x+2,  y ),
                  (x-1, y+1), (x, y+1), (x+1, y+1), (x+2, y+1),
                  (x-1, y+2), (x, y+2), (x+1, y+2), (x+2, y+2)],
            1 => [(x-1, y-1), (x, y-1), (x+1, y-1),
                  (x-1,  y ), (x,  y ), (x+1,  y ), (x+2,  y ),
                  (x-1, y+1), (x, y+1), (x+1, y+1), (x+2, y+1),
                  (x-1, y+2), (x, y+2), (x+1, y+2),           ],
            2 => [(x-1, y-1), (x, y-1), (x+1, y-1), (x+2, y-1),
                  (x-1,  y ), (x,  y ), (x+1,  y ), (x+2,  y ),
                  (x-1, y+1), (x, y+1), (x+1, y+1), (x+2, y+1),
                              (x, y+2), (x+1, y+2),           ],
            3 => [            (x, y-1), (x+1, y-1), (x+2, y-1),
                  (x-1,  y ), (x,  y ), (x+1,  y ), (x+2,  y ),
                  (x-1, y+1), (x, y+1), (x+1, y+1), (x+2, y+1),
                              (x, y+2), (x+1, y+2), (x+2, y+2)],
            _ => [(-1, -1) ; 14],
        }
    }
    /* Returns the reward from taking an action */
    pub fn perform_action(&mut self, a: Action) -> f32 {
        /* Calculate new positions */
        let (mut nx, mut ny, mut ndirn) = match a {
            Action::FORWARD => match self.dirn {
                0 => (self.x,     self.y - 1, self.dirn),
                1 => (self.x + 1, self.y,     self.dirn),
                2 => (self.x,     self.y + 1, self.dirn),
                3 => (self.x - 1, self.y,     self.dirn),
                _ => (self.x,     self.y,     self.dirn),
            },
            Action::REVERSE => match self.dirn {
                0 => (self.x,     self.y + 1, self.dirn),
                1 => (self.x - 1, self.y,     self.dirn),
                2 => (self.x,     self.y - 1, self.dirn),
                3 => (self.x + 1, self.y,     self.dirn),
                _ => (self.x,     self.y,     self.dirn),
            },
            Action::L => (self.x, self.y, (self.dirn - 1) & 0x3),
            Action::R => (self.x, self.y, (self.dirn + 1) & 0x3),
            Action::SUCK => (self.x, self.y, self.dirn),
        };
        let r =
            if a != Action::SUCK {
                /* Check for collisions with obstacles and hazards */
                let mut penalty = 0.0;
                for (x, y) in Room::get_occupied_squares(nx, ny, ndirn) {
                    if x >= 0 && x < self.xsize && y >= 0 && y < self.ysize {
                        let v = self.board[(y * self.xsize + x) as usize];
                        penalty = if v == -2 && penalty > -20.0 { -20.0 } else { penalty };
                        penalty = if v == -3 && penalty > -50.0 { -50.0 } else { penalty };
                    } else {
                        penalty = if penalty > -20.0 { -20.0 } else { penalty };
                    }
                }
                if penalty < 0.0 {
                    (nx, ny, ndirn) = (self.x, self.y, self.dirn);
                }
                /* Apply movement penalty for forward (-0.1), reverse (-2.0) and rotation (-0.2) */
                if a == Action::FORWARD {
                    penalty -= 0.05;
                } else if a == Action::REVERSE {
                    penalty -= 1.0;
                } else {
                    penalty -= 0.1;
                }
                penalty
            } else {
                if self.x == 1 && self.y == self.ysize - 3 {
                    /* Robot is on charging pad, start a new level */
                    self.generate_level();
                    -0.2
                } else {
                    /* For every square covered by the vacuum, reduce dirt level by 1
                     * Reward 1 for each dirt removed this way, minus a constant -0.1 */
                    self.get_suction_range().iter().filter(|(x, y)| {
                        if *x >= 0 && *x < self.xsize && *y >= 0 && *y < self.ysize
                            && self.board[(y * self.xsize + x) as usize] > 0 {
                            self.board[(y * self.xsize + x) as usize] -= 1;
                            true
                        } else {
                            false
                        }
                    }).collect::<Vec<&(i32, i32)>>().len() as f32 * 1.0 - 0.1
                }
            }
        ;

        /* Apply movement */
        (self.x, self.y, self.dirn) = (nx, ny, ndirn);

        self.r += r;
        r
    }
    pub fn get_nn_input(&self) -> RoomVec {
        /* Returns an input vector (len = SIZE_STATE = 403) for a neural network:
         * - Values for a 20 x 20 space around the robot
         * - Coordinates (x, y) of the robot, relative to the charging pad
         * - Direction in which the robot is facing */
        std::array::from_fn(|i| {
            match i {
                0..=399 => {
                    let (x, y): (i32, i32) = (i as i32 % 20, i as i32 / 20);
                    let (x, y): (i32, i32) = (self.x + x - 9, self.y + y - 9);
                    if x >= 0 && x < self.xsize && y >= 0 && y < self.ysize {
                        self.board[(y * self.xsize + x) as usize] as f32
                    } else {
                        /* Out of bounds, return an obstacle */
                        -2.0
                    }
                }
                400 => (self.x - 1) as f32,
                401 => (self.ysize - self.y - 3) as f32,
                402 => self.dirn as f32,
                _   => 0.0, /* Should not occur */
            }
        })
    }
    pub fn get_total_reward(&self) -> f32 {
        self.r
    }
    pub fn draw(&self, first_time: bool) -> Result<()> {
        /* If we're on the charging pad, it could be a new level */
        let redraw_map = self.x == 1 && self.y == self.ysize - 3;

        let mut stdout = stdout();
        let draw_xmin;
        let draw_xmax;
        let draw_ymin;
        let draw_ymax;
        if first_time {
            queue!(stdout, terminal::Clear(terminal::ClearType::All))?;
        }
        if first_time || redraw_map {
            draw_xmin = 0;
            draw_xmax = self.xsize;
            draw_ymin = 0;
            draw_ymax = self.ysize;
        } else {
            /* Just redraw an area around the robot. */
            draw_xmin = if self.x - 5 >= 0 {self.x - 5} else {0};
            draw_xmax = if self.x + 5 < self.xsize {self.x + 5} else {self.xsize};
            draw_ymin = if self.y - 5 >= 0 {self.y - 5} else {0};
            draw_ymax = if self.y + 5 < self.ysize {self.y + 5} else {self.ysize};
        }

        let suction_range = self.get_suction_range();

        /* Draw room features */
        for x in draw_xmin..draw_xmax {
            for y in draw_ymin..draw_ymax {
                let i: usize = (y * self.xsize + x) as usize;
                let (xscr, yscr) = ((2 * x + 1) as u16, (y + 1) as u16);
                queue!(stdout, cursor::MoveTo(xscr, yscr))?;
                if suction_range.contains(&(x, y)) {
                    queue!(stdout, SetBackgroundColor(Color::AnsiValue(236)))?;
                }
                match &self.board[i] {
                    -3 => { queue!(stdout, SetForegroundColor(Color::Red), Print("!!"))?; }
                    -2 => { queue!(stdout, SetForegroundColor(Color::Yellow), Print("XX"))?; }
                    -1 => { queue!(stdout, SetForegroundColor(Color::Cyan), Print("OO"))?; }
                    0 => { queue!(stdout, Print("  "))?; }
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
        match self.dirn {
            0 => {
                /* UP */
                queue!(stdout,
                       cursor::MoveTo(xorig, yorig - 1),
                       Print("####"),
                       cursor::MoveTo(xorig - 2, yorig),
                       Print("|      |"),
                       cursor::MoveTo(xorig - 2, yorig + 1),
                       Print("| ╖  ╓ |"),
                       cursor::MoveTo(xorig - 1, yorig + 2),
                       Print("`----'"))?;
            }
            1 => {
                /* RIGHT */
                queue!(stdout,
                       cursor::MoveTo(xorig - 1, yorig - 1),
                       Print(".----"),
                       cursor::MoveTo(xorig - 2, yorig),
                       Print("| ╛    #"),
                       cursor::MoveTo(xorig - 2, yorig + 1),
                       Print("| ╕    #"),
                       cursor::MoveTo(xorig - 1, yorig + 2),
                       Print("`----"))?;
            }
            2 => {
                /* DOWN */
                queue!(stdout,
                       cursor::MoveTo(xorig - 1, yorig - 1),
                       Print(",----."),
                       cursor::MoveTo(xorig - 2, yorig),
                       Print("| ╜  ╙ |"),
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
                       Print("#    ╘ |"),
                       cursor::MoveTo(xorig - 2, yorig + 1),
                       Print("#    ╒ |"),
                       cursor::MoveTo(xorig, yorig + 2),
                       Print("----'"))?;
            }
            _ => {}
        }
        queue!(stdout, ResetColor)?;

        if first_time {
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
            /* Information */
            queue!(stdout, cursor::MoveTo(0, (self.ysize + 2) as u16), Print("Score:"))?;
            queue!(stdout, cursor::MoveTo(0, (self.ysize + 4) as u16),
                    SetForegroundColor(Color::Cyan), Print("Forward"),
                    ResetColor, Print(": Up arrow | "),
                    SetForegroundColor(Color::Cyan), Print("Back"),
                    ResetColor, Print(": Down arrow | "),
                    SetForegroundColor(Color::Cyan), Print("Turn CCW"),
                    ResetColor, Print(": Left arrow | "),
                    SetForegroundColor(Color::Cyan), Print("Turn CW"),
                    ResetColor, Print(": Right arrow"),
                    cursor::MoveTo(0, (self.ysize + 5) as u16),
                    SetForegroundColor(Color::Cyan), Print("Suck"), ResetColor,
                    Print(": Spacebar (doing so over the charging pad advances to the next level)")
                    )?;
            queue!(stdout, cursor::MoveTo(0, (self.ysize + 7) as u16), SetForegroundColor(Color::Magenta),
                    Print("Quit"), ResetColor, Print(": ESC or Q"))?;
        }
        /* Information */
        queue!(stdout, cursor::MoveTo(7, (self.ysize + 2) as u16), Print(format!("{:7.1}", self.r)))?;

        stdout.flush()?;
        Ok(())
    }
}


