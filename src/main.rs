extern crate ncurses;

use std::collections::HashMap;
use rand::Rng;

struct TermColorPair {
    id : i16,
}

struct TermColor {
    nextid : i16,
    pairs : HashMap<String, TermColorPair>,
}

/* Wrapper around ncurses color pairs */
impl TermColorPair {
    fn new(id: i16, bg: i16, fg: i16) -> TermColorPair {
        ncurses::init_pair(id, bg, fg);
        TermColorPair{id}
    }
    fn addstr(&self, x: i32, y: i32, s: &str) {
        ncurses::attron(ncurses::COLOR_PAIR(self.id.try_into().unwrap()));
        ncurses::mvaddstr(y, x, s);
        ncurses::attroff(ncurses::COLOR_PAIR(self.id.try_into().unwrap()));
    }
    fn addc(&self, x: i32, y: i32, c: char) {
        ncurses::attron(ncurses::COLOR_PAIR(self.id.try_into().unwrap()));
        ncurses::mvaddch(y, x, c as u32);
        ncurses::attroff(ncurses::COLOR_PAIR(self.id.try_into().unwrap()));
    }
}

impl TermColor {
    fn new() -> TermColor {
        TermColor{nextid : 1, pairs : HashMap::new()}
    }
    fn add_pair(&mut self, name: &str, bg: i16, fg: i16) {
        let tcp = TermColorPair::new(self.nextid, bg, fg);
        self.pairs.insert(String::from(name), tcp);
        self.nextid += 1;
    }
    fn get(&self, name: &str) -> &TermColorPair {
        self.pairs.get(name).unwrap()
    }
}

struct Room {
    xsize: i32,
    ysize: i32,
    dust : Vec<(i32, i32)>,
}

impl Room {
    fn new(xsize: i32, ysize: i32) -> Room {
        let mut rng = rand::thread_rng();
        let mut dust: Vec<(i32, i32)> = Vec::new();
        for i in 0..((xsize * ysize) / 10) {
            let x = rng.gen_range(0..xsize);
            let y = rng.gen_range(0..ysize);
            dust.push((x, y));
        }
        Room{xsize, ysize, dust}
    }
}

fn main() {
    let mut win = ncurses::initscr();
    ncurses::start_color();

    let (w, h) = (ncurses::getmaxx(win), ncurses::getmaxy(win));
    
    /* Colors */
    let mut tc = TermColor::new();
    tc.add_pair("green", ncurses::COLOR_GREEN, ncurses::COLOR_BLACK);
    tc.add_pair("red", ncurses::COLOR_RED, ncurses::COLOR_BLACK);
    
    /* Game window */
    let game_win = ncurses::newwin(h - 6, w, 0, 0);
    ncurses::wborder(game_win, 0, 0, 0, 0, 0, 0, 0, 0);
    ncurses::refresh();
    ncurses::wrefresh(game_win);

    tc.get("green").addstr(1, 0, "Robot Vacuum Simulator!");

    let room = Room::new(w - 2, h - 8);

    for (x, y) in room.dust {
        tc.get("red").addc(x + 1, y + 1, 'x');
    }
    
    while true {
        let c = ncurses::getch();
    }
    
}
