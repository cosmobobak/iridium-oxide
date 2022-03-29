#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(dead_code)]

use std::io::Write;

use crate::{
    gamerunner::{GameRunner, Player},
    mcts::{Behaviour, Limit, RolloutPolicy, MCTS}, connectfour::{Connect4},
};

mod agent;
mod connectfour;
mod constants;
mod elo;
mod game;
mod gamerunner;
mod iterbits;
mod mcts;
mod searchtree;
mod tictactoe;
mod treenode;
mod uct;
mod gomoku;
mod datageneration;
// mod ultimatetictactoe;

#[allow(unused_imports)]
use Player::{Computer, Human};

const GAMES: usize = 1;

fn fastplay(config: Behaviour) {
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).unwrap();
    let buf = buf.trim();
    match buf.chars().next() {
        Some('2') => GameRunner::<Connect4>::new(Human, Computer(MCTS::new(config))).run(),
        Some('1') => GameRunner::<Connect4>::new(Computer(MCTS::new(config)), Human).run(),
        _ => panic!("invalid input: \"{buf}\""),
    }
}

fn main() {
    // get the command line arguments
    let args: Vec<String> = std::env::args().collect();
    assert!(args.len() >= 2);
    let games = args[1].parse::<usize>().unwrap();

    println!("iridium-oxide operating at full capacity!");
    println!("{games} games will be played");

    generate_data(games);
}

fn generate_data(games: usize) {
    let limit = Limit::Rollouts(10_000);
    let config = Behaviour {
        debug: false,
        readout: true,
        limit,
        root_parallelism_count: 1,
        rollout_policy: RolloutPolicy::Random,
        exp_factor: 5.0,
    };

    let episode_data = (0..games).map(|it| { 
        // print progress bar   
        print!("{}%     \r", it * 100 / games);
        std::io::stdout().flush().unwrap();
        GameRunner::<Connect4>::play_training_game(config)
    }).reduce(|a, b| a + b).expect("failed to generate training data");
    println!("100%");
    episode_data.save::<Connect4>("connect4data.csv").expect("failed to write file");
}
