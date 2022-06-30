#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::{io::Write, time::Instant};

use crate::{
    gamerunner::{GameRunner, Player},
    games::{connectfour::Connect4, gomoku::Gomoku, tictactoe::TicTacToe},
    mcts::{Behaviour, Limit, RolloutPolicy, MCTS},
};

mod agent;
mod constants;
mod datageneration;
mod elo;
mod game;
mod gamerunner;
mod games;
mod iterbits;
mod mcts;
mod searchtree;
mod treenode;
mod ucb;

use datageneration::VectoriseState;
use game::Game;
use Player::{Computer, Human};

const NAME: &str = "iridium-oxide";

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("play") => {
            let game = args.get(2);
            match game.map(String::as_str) {
                Some("connect4") => play::<Connect4>(
                    &Behaviour::default(),
                    args.get(3)
                        .unwrap_or_else(|| panic!("No side provided."))
                        .parse()
                        .unwrap(),
                ),
                Some("tictactoe") => play::<TicTacToe>(
                    &Behaviour::default(),
                    args.get(3)
                        .unwrap_or_else(|| panic!("No side provided."))
                        .parse()
                        .unwrap(),
                ),
                Some("gomoku9") => play::<Gomoku<9>>(
                    &Behaviour::default(),
                    args.get(3)
                        .unwrap_or_else(|| panic!("No side provided."))
                        .parse()
                        .unwrap(),
                ),
                Some("gomoku13") => play::<Gomoku<13>>(
                    &Behaviour::default(),
                    args.get(3)
                        .unwrap_or_else(|| panic!("No side provided."))
                        .parse()
                        .unwrap(),
                ),
                Some("gomoku19") => play::<Gomoku<19>>(
                    &Behaviour::default(),
                    args.get(3)
                        .unwrap_or_else(|| panic!("No side provided."))
                        .parse()
                        .unwrap(),
                ),
                Some("reversi" | "uttt") => todo!(),
                Some(unknown) => {
                    if unknown != "help" {
                        eprintln!("Unknown game: {unknown}");
                    }
                    println!(
                        "Available games: connect4, tictactoe, gomoku{{9,13,19}}, reversi, uttt"
                    );
                }
                None => {
                    println!(
                        "Available games: connect4, tictactoe, gomoku{{9,13,19}}, reversi, uttt"
                    );
                }
            }
        }
        Some("generate") => {
            let game = args.get(2);
            let games = args[3].parse().unwrap();
            let fname = args[4].as_str();
            println!("{games} games will be played");
            let start = Instant::now();
            match game.map(String::as_str) {
                Some("connect4") => generate_data::<Connect4>(games, fname),
                Some("tictactoe") => generate_data::<TicTacToe>(games, fname),
                Some("gomoku9" | "gomoku13" | "gomoku19") => unimplemented!(),
                Some("reversi" | "uttt") => todo!(),
                Some(unknown) => {
                    if unknown != "help" {
                        eprintln!("Unknown game: {unknown}");
                    }
                    println!(
                        "Available games: connect4, tictactoe, gomoku{{9,13,19}}, reversi, uttt"
                    );
                    return;
                }
                None => {
                    println!(
                        "Available games: connect4, tictactoe, gomoku{{9,13,19}}, reversi, uttt"
                    );
                    return;
                }
            }
            let secs = start.elapsed().as_secs_f64();
            println!("Generating data took {secs:.2} seconds");
        }
        None => {
            println!("Available commands:");
            println!("1. Play against a the computer ({NAME} play <game> <1|2>)");
            println!("2. Generate data for a game ({NAME} generate <game> <count> <fname>)");
        }
        Some(unknown) => {
            if unknown != "help" {
                eprintln!("Unknown command: {unknown}");
            }
            println!("Available commands:");
            println!("1. Play against a the computer ({NAME} play <game> <1|2>)");
            println!("2. Generate data for a game ({NAME} generate <game> <count> <fname>)");
        }
    }
}

fn play<G: Game>(config: &Behaviour, player: usize) {
    println!("iridium-oxide operating at full capacity!");
    match player {
        1 => GameRunner::<G>::new(Human, Computer(MCTS::new(config))).run(),
        2 => GameRunner::<G>::new(Computer(MCTS::new(config)), Human).run(),
        _ => panic!("fastplay: player must be 1 (you play first) or 2 (i play first)"),
    }
}

fn generate_data<G: VectoriseState>(games: u32, fname: &str) {
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

    let episode_data = (0..games)
        .map(|it| {
            // print progress bar
            print!("{:.2}%     \r", f64::from(it) * 100.0 / f64::from(games));
            std::io::stdout().flush().unwrap();
            GameRunner::<G>::play_training_game(&config)
        })
        .reduce(|a, b| a + b)
        .expect("failed to generate training data");
    println!("100%     ");
    episode_data
        .save::<G>(&format!("datasets/{fname}.csv"))
        .expect("failed to write file");
    episode_data.summary();
}
