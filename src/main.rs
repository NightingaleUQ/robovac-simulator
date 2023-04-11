mod game;

use std::env;

use std::io::{
    stdout,
};

use crossterm::{
    event, event::Event,
    terminal, cursor,
    ExecutableCommand,
};

use tch::{
    nn, nn::Sequential, nn::Module, nn::OptimizerConfig,
    Device, Tensor, Reduction
};

use rand::Rng;

use game::{
    Room, RoomVec
};

const NUM_EPISODES: usize = 32;
const EPISODE_LEN: usize = 1024;
const BATCH_SIZE: usize = 256;

/* Structure of our network */
fn net(vs: &nn::Path) -> Sequential {
    nn::seq()
        .add(nn::linear(vs, game::SIZE_STATE as i64, 256, Default::default()))
        .add_fn(|x| x.relu())
        .add(nn::linear(vs, 256, 128, Default::default()))
        .add_fn(|x| x.relu())
        .add(nn::linear(vs, 128, 128, Default::default()))
        .add_fn(|x| x.relu())
        .add(nn::linear(vs, 128, 64, Default::default()))
        .add_fn(|x| x.relu())
        .add(nn::linear(vs, 64, game::SIZE_ACTION as i64, Default::default()))
}

struct SARS {
    /* State, Action, Reward, Next state */
    s:      RoomVec,
    a:      usize,
    r:      f32,
    s_next: RoomVec, 
}

fn main() {
    let (termw, termh) = terminal::size().unwrap();
    let (w, h): (i32, i32) = (((termw - 2) / 2) as i32, (termh - 8) as i32);

    let args: Vec<String> = env::args().collect();
    
    /* Neural network parameters */
    let dev = Device::cuda_if_available();
    let vs = nn::VarStore::new(dev);
    let net = net(&vs.root());

    /* Mode of control
     * 0: User input mode
     * 1: Neural network input "demo" mode
     */
    let input_mode;

    if args.len() >= 2 && String::from("demo") == args[1] {
        /* Neural network demo mode */
        println!("Initialisaing training...");
        let mut opt = nn::Adam::default().build(&vs, 1e-3).expect("Failed to build optimiser");
        let mut rng = rand::thread_rng();
        println!("CUDA available? {}", dev.is_cuda());

        /* Replay memory */
        let mut rmem = Vec::<SARS>::with_capacity(NUM_EPISODES * EPISODE_LEN);
        for ep in 0..NUM_EPISODES {
            let mut room = Room::new(w, h);
            for _ in 0..EPISODE_LEN {
                let s = room.get_nn_input();
                /* Epsilon-greedy action selection */
                let a = if rng.gen::<f32>() < 0.6 {
                    rng.gen_range(0..game::SIZE_ACTION)
                } else {
                    get_nn_best_action(&net, &s)
                };
                print!("{} ", a);
                let r = room.perform_action(game::i_to_act(a));
                let s_next = room.get_nn_input();
                rmem.push(SARS{s, a, r, s_next});
                if rmem.len() >= BATCH_SIZE {
                    /* Sample from memory and learn */
                    let sample = rand::seq::index::sample(&mut rng, rmem.len(), BATCH_SIZE)
                        .iter().map(|i| &rmem[i]).collect::<Vec<&SARS>>();
                    let mut s = Vec::with_capacity(game::SIZE_STATE * BATCH_SIZE);
                    let mut a = Vec::with_capacity(BATCH_SIZE);
                    let mut r = Vec::with_capacity(BATCH_SIZE);
                    let mut s_next = Vec::with_capacity(game::SIZE_STATE * BATCH_SIZE);
                    for sars in sample {
                        s.extend(sars.s);
                        a.push(sars.a);
                        r.push(sars.r);
                        s_next.extend(sars.s_next);
                    }
                    let fwd = net.forward(&Tensor::of_slice(&s).view((BATCH_SIZE as i64, game::SIZE_STATE as i64)));
                    let q_next = net.forward(&Tensor::of_slice(&s_next).view((BATCH_SIZE as i64, game::SIZE_STATE as i64)));
                    let max_next = Vec::from(q_next).chunks(game::SIZE_ACTION).into_iter().map(|slice| {
                        let mut max = std::f32::MIN;
                        for val in slice.iter() {
                            max = if *val > max {*val} else {max}
                        }
                        max
                    }).collect::<Vec<f32>>();
                    let model_r: Tensor = Tensor::of_slice(&r) + 0.999 * Tensor::of_slice(&max_next);
                    /* Modified forward tensor with expected reward values */
                    let mut y: Vec<f32> = Vec::from(&fwd);
                    for (i, chunk) in y.chunks_mut(game::SIZE_ACTION).into_iter().enumerate() {
                        chunk[a[i]] = Vec::from(&model_r)[i];
                    }
                    let y = Tensor::of_slice(&y).view((BATCH_SIZE as i64, game::SIZE_ACTION as i64));
                    let loss = fwd.mse_loss(&y, Reduction::Sum);
                    print!("({}) ", Vec::from(&loss).iter().sum::<f32>()); /* Av. loss */
                    opt.backward_step(&loss);
                }
            }
            println!("");
            let temp = room.get_nn_input();
            for chunk in temp.chunks(20).into_iter() {
                println!("{:2.0?}", chunk);
            }
            println!("Episode {:3} (of {}) Reward: {:7.1}", ep + 1, NUM_EPISODES, room.get_total_reward());
        }
        input_mode = 1;
    } else {
        input_mode = 0;
    }

    /* Gameplay loop */

    if input_mode == 0 {
        terminal::enable_raw_mode().expect("Failed to enable RAW mode.");
    }
    _ = stdout().execute(cursor::Hide);

    let mut room = Room::new(w, h);
    _ = room.draw(true);

    loop {
        let a: isize =
            if input_mode == 0 {
                get_action_user()
            } else {
                get_action_nn(&net, &room.get_nn_input())
            };
        if a == -1 {
            break;
        }
        room.perform_action(game::i_to_act(a as usize));
        _ = room.draw(false);
    }
    _ = stdout().execute(terminal::Clear(terminal::ClearType::All));
    _ = stdout().execute(cursor::Show);
}

fn get_nn_best_action(net: &Sequential, s: &RoomVec) -> usize {
    let mut max = std::f32::MIN;
    let mut argmax = 0;
    let out = Vec::from(net.forward(&Tensor::of_slice(s)));
    for (i, v) in out.iter().enumerate() {
        if *v > max {
            max = *v;
            argmax = i;
        }
    }
    argmax
}

fn get_action_nn(net: &Sequential, s: &RoomVec) -> isize {
    /* Epsilon-greedy action selection */
    let mut rng = rand::thread_rng();
    if rng.gen::<f32>() < 0.05 {
        rng.gen_range(0..game::SIZE_ACTION) as isize
    } else {
        get_nn_best_action(net, s) as isize
    }
}

fn get_action_user() -> isize {
    let read = event::read().unwrap();
    loop {
        match read {
            Event::Key(event::KeyEvent{code: event::KeyCode::Up, ..})
                => return 0,
            Event::Key(event::KeyEvent{code: event::KeyCode::Down, ..})
                => return 1,
            Event::Key(event::KeyEvent{code: event::KeyCode::Left, ..})
                => return 2,
            Event::Key(event::KeyEvent{code: event::KeyCode::Right, ..})
                => return 3,
            Event::Key(event::KeyEvent{code: event::KeyCode::Char(' '), ..})
                => return 4,
            Event::Key(event::KeyEvent{code: event::KeyCode::Esc, ..}) |
            Event::Key(event::KeyEvent{code: event::KeyCode::Char('q'), ..})
                => return -1, 
            _
                => { continue; }
        }
    }
}
