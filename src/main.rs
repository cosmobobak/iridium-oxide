#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(dead_code)]



use crate::{
    gamerunner::{GameRunner, Player},
    mcts::{Behaviour, Limit, RolloutPolicy}, connectfour::{Connect4},
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

#[allow(unused_imports)]
use Player::{Computer, Human};

fn main() {
    println!("iridium-oxide operating at full capacity!");

    let limit = Limit::Rollouts(100_000);

    let config = Behaviour {
        debug: false,
        readout: false,
        limit,
        root_parallelism_count: 1,
        rollout_policy: RolloutPolicy::Random,
        exp_factor: 10.0,
    };
    
    let data = GameRunner::<Connect4>::play_training_game(config);

    println!("{:?}", data);
}
