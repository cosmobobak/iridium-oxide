#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(dead_code)]

use std::{io::Write, time::Instant};

use crate::{
    gamerunner::{GameRunner, Player},
    mcts::{Behaviour, Limit, RolloutPolicy, MCTS}, games::connectfour::Connect4,
};

mod games;
mod agent;
mod constants;
mod elo;
mod game;
mod gamerunner;
mod iterbits;
mod mcts;
mod searchtree;
mod treenode;
mod ucb;
mod datageneration;

#[allow(unused_imports)]
use Player::{Computer, Human};
use game::Game;

const GAMES: usize = 1;

fn fastplay<G: Game>(config: &Behaviour) {
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).unwrap();
    let buf = buf.trim();
    match buf.chars().next() {
        Some('2') => GameRunner::<G>::new(Human, Computer(MCTS::new(config))).run(),
        Some('1') => GameRunner::<G>::new(Computer(MCTS::new(config)), Human).run(),
        _ => panic!("invalid input: \"{buf}\""),
    }
}

fn main() {
    // let config = Behaviour {
    //     debug: false,
    //     readout: true,
    //     limit: Limit::Time(std::time::Duration::from_secs(3)),
    //     root_parallelism_count: 1,
    //     rollout_policy: RolloutPolicy::Decisive,
    //     exp_factor: DEFAULT_EXP_FACTOR,
    //     training: false,
    // };
    
    // get the command line arguments
    let args: Vec<String> = std::env::args().collect();
    assert!(args.len() >= 2, "pass the number of games to play as a CLI argument");
    let games = args[1].parse::<u32>().unwrap();

    println!("iridium-oxide operating at full capacity!");
    println!("{games} games will be played");

    let start = Instant::now();
    generate_data(games);
    let elapsed = start.elapsed();
    println!("Generating data took {secs:.2} seconds", secs = elapsed.as_secs_f64());
}

fn generate_data(games: u32) {
    let limit = Limit::Rollouts(500_000);
    let config = Behaviour {
        debug: false,
        readout: false,
        limit,
        root_parallelism_count: 1,
        rollout_policy: RolloutPolicy::Random,
        exp_factor: 5.0,
        training: true,
    };

    let episode_data = (0..games).map(|it| { 
        // print progress bar   
        print!("{:.2}%     \r", f64::from(it) * 100.0 / f64::from(games));
        std::io::stdout().flush().unwrap();
        GameRunner::<Connect4>::play_training_game(&config)
    }).reduce(|a, b| a + b).expect("failed to generate training data");
    println!("100%     ");
    episode_data.save::<Connect4>("datasets/connect4data.csv").expect("failed to write file");
    episode_data.summary();
}
