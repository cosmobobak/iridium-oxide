#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(dead_code)]

use std::time::Duration;

use crate::{
    connectfour::Connect4,
    gamerunner::{GameRunner, Player},
    mcts::{Behaviour, Limit, MCTS, RolloutPolicy},
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

#[allow(unused_imports)]
use Player::{Computer, Human};

fn main() {
    println!("iridium-oxide operating at full capacity!");

    // let limit = Limit::Time(Duration::from_millis(300));
    let limit = Limit::Rollouts(10_000);

    let engine1 = MCTS::new(Behaviour {
        debug: true,
        readout: true,
        limit,
        root_parallelism_count: 1,
        rollout_policy: RolloutPolicy::Decisive,
    });

    let engine2 = MCTS::new(Behaviour {
        debug: true,
        readout: true,
        limit,
        root_parallelism_count: 1,
        rollout_policy: RolloutPolicy::Random,
    });

    GameRunner::<Connect4>::new(Computer(engine1), Computer(engine2)).play_match(100);
}
