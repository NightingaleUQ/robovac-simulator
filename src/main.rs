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
    nn, nn::Module, nn::OptimizerConfig, Device, Tensor, Reduction
};

use rand::{
    seq::IteratorRandom, Rng
};

use game::{
    Room, Action, RoomVec
};

const NUM_EPISODES: usize = 1024;
const EPISODE_LEN: usize = 512;
const BATCH_SIZE: usize = 256;

/* Structure of our network */
fn net(vs: &nn::Path) -> impl Module {
    nn::seq()
        .add(nn::linear(vs / "layer1", 403, 256, Default::default()))
        .add_fn(|x| x.sigmoid())
        .add(nn::linear(vs / "layer2", 256, 128, Default::default()))
        .add_fn(|x| x.sigmoid())
        .add(nn::linear(vs / "layer3", 128, 64, Default::default()))
        .add_fn(|x| x.sigmoid())
        .add(nn::linear(vs, 64, 5, Default::default()))
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
    if args.len() >= 2 && String::from("demo") == args[1] {
        /* Neural network demo mode */
        println!("Initialisaing training...");
        let dev = Device::cuda_if_available();
        let vs = nn::VarStore::new(dev);
        let net = net(&vs.root());
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
                let a = if rng.gen::<f32>() < 0.1 {
                    rng.gen_range(0..5)
                } else {
                    let mut max = std::f32::MIN;
                    let mut argmax = 0;
                    let out = Vec::from(net.forward(&Tensor::of_slice(&s)));
                    for (i, v) in out.iter().enumerate() {
                        if *v > max {
                            max = *v;
                            argmax = i;
                        }
                    }
                    argmax
                };
                print!("{} ", a);
                let r = room.perform_action(game::i_to_act(a));
                let s_next = room.get_nn_input();
                rmem.push(SARS{s, a, r, s_next});
                if rmem.len() >= BATCH_SIZE {
                    /* Sample from memory and learn */
                    let sample = rmem.iter().choose_multiple(&mut rng, BATCH_SIZE);
                    let mut s = Vec::with_capacity(403 * BATCH_SIZE);
                    let mut a = Vec::with_capacity(BATCH_SIZE);
                    let mut r = Vec::with_capacity(BATCH_SIZE);
                    let mut s_next = Vec::with_capacity(BATCH_SIZE);
                    for sars in sample {
                        s.extend(sars.s);
                        a.push(sars.a);
                        r.push(sars.r);
                        s_next.extend(sars.s_next);
                    }
                    let fwd = net.forward(&Tensor::of_slice(&s).view((BATCH_SIZE as i64, 403)));
                    let q_next = net.forward(&Tensor::of_slice(&s_next).view((BATCH_SIZE as i64, 403)));
                    let max_next = Vec::from(q_next).chunks(5).into_iter().map(|slice| {
                        let mut max = std::f32::MIN;
                        for val in slice.iter() {
                            max = if *val > max {*val} else {max}
                        }
                        max
                    }).collect::<Vec<f32>>();
                    let model_r: Tensor = Tensor::of_slice(&r) + 0.98 * Tensor::of_slice(&max_next);
                    /* Modified forward tensor with expected reward values */
                    let mut y: Vec<f32> = Vec::from(&fwd);
                    for (i, chunk) in y.chunks_mut(5).into_iter().enumerate() {
                        chunk[a[i]] = Vec::from(&model_r)[i];
                    }
                    let y = Tensor::of_slice(&y).view((BATCH_SIZE as i64, 5));
                    let loss = fwd.mse_loss(&y, Reduction::Sum).set_requires_grad(false);
                    print!("({}) ", Vec::from(&loss).iter().sum::<f32>()); /* Av. loss */
                    opt.backward_step(&loss);
                }
            }
            println!("");
            let temp = room.get_nn_input();
            for chunk in temp.chunks(20).into_iter() {
                println!("{:4?}", chunk);
            }
            println!("Episode {:3} Reward: {:7.1}", ep, room.get_total_reward());
        }
    } else {
        terminal::enable_raw_mode().expect("Failed to enable RAW mode.");
        _ = stdout().execute(cursor::Hide);

        /* Interactive player mode */
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
        }}
    _ = stdout().execute(terminal::Clear(terminal::ClearType::All));
    _ = stdout().execute(cursor::Show);
}
