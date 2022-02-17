#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(dead_code)]

use std::time::Duration;

use crate::{
    connectfour::Connect4,
    gamerunner::{GameRunner, Player},
    mcts::{Behaviour, Limit, MonteCarloTreeSearcher},
};

mod agent;
mod connectfour;
mod constants;
mod game;
mod elo;
mod gamerunner;
mod mcts;
mod searchtree;
mod tictactoe;
mod treenode;
mod uct;

fn main() {
    use Player::Computer;

    println!("iridium-oxide operating at full capacity!");

    let mcts1 = MonteCarloTreeSearcher::new(Behaviour {
        debug: false,
        readout: false,
        limit: Limit::Time(Duration::from_millis(10)),
    });

    let mcts2 = MonteCarloTreeSearcher::new(Behaviour {
        debug: false,
        readout: false,
        limit: Limit::Time(Duration::from_millis(10)),
    });

    GameRunner::<Connect4>::new(Computer(mcts1), Computer(mcts2)).play_match(1000);
}
