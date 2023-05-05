#![allow(dead_code)]

use std::io::Write;

use crate::{
    agent::Agent,
    elo,
    game::{Game, MoveBuffer},
    mcts::{MCTSExt, MCTS},
};

#[derive(Clone)]
pub enum Player<'a, G: Game> {
    Human,
    Computer(MCTS<'a, G>),
}

impl<G: Game + MCTSExt> Agent<G> for Player<'_, G> {
    fn transition(&mut self, state: G) -> G {
        let mut state = state;
        match self {
            Self::Human => {
                let mut buffer = G::Buffer::default();
                state.generate_moves(&mut buffer);
                println!("Your options are:");
                println!("{buffer}");
                let user_move = loop {
                    print!("Enter move: ");
                    std::io::stdout().flush().unwrap();
                    let mut user_input = String::new();
                    std::io::stdin().read_line(&mut user_input).unwrap();
                    let user_input = user_input.trim().to_uppercase();
                    let needle = buffer
                        .iter()
                        .find(|&&m| format!("{m}").to_uppercase() == user_input);
                    if let Some(needle) = needle {
                        break *needle;
                    }
                };

                state.push(user_move);
                state
            }
            Self::Computer(agent) => agent.best_next_board(&state),
        }
    }
}

pub struct GameRunner<'a, G: Game> {
    players: [Player<'a, G>; 2],
}

impl<'a, G: Game + Default + MCTSExt> GameRunner<'a, G> {
    pub const fn new<'b, 'c>(player1: Player<'b, G>, player2: Player<'c, G>) -> Self
    where
        'b: 'a,
        'c: 'a,
    {
        Self {
            players: [player1, player2],
        }
    }

    fn do_printout(&self) -> bool {
        self.players.iter().any(|p| matches!(p, Player::Human))
    }

    pub fn run(&mut self) {
        self.run_with(G::default());
    }

    pub fn run_with(&mut self, state: G) {
        let mut state = state;
        while !state.is_terminal() {
            if self.do_printout() {
                println!("{state}");
            }
            let player = match state.turn() {
                1 => &mut self.players[0],
                -1 => &mut self.players[1],
                _ => panic!("Invalid turn"),
            };
            state = player.transition(state);
            if self.do_printout() {
                println!();
            }
        }
        if self.do_printout() {
            println!("{state}");
            println!("{}", state.outcome().unwrap());
        }
    }

    /// Returns the result of the encounter, where 1 means X won, -1 means the O won, and 0 means a draw.
    /// The `flip` parameter indicates whether the players are flipped.
    fn do_encounter(players: &mut [Player<G>; 2], flip: bool) -> i8 {
        let mut state = G::default();
        let alternator = if flip { -1 } else { 1 };
        while !state.is_terminal() {
            let turn = state.turn() * alternator;
            let player = match turn {
                1 => &mut players[0],
                -1 => &mut players[1],
                _ => panic!("Invalid turn"),
            };
            state = player.transition(state);
        }
        state.evaluate()
    }

    pub fn play_match(&mut self, games: usize) {
        const RED: &str = "\u{001b}[31m";
        const GREEN: &str = "\u{001b}[32m";
        const RESET: &str = "\u{001b}[0m";

        println!("Running a {games}-game match...");
        assert_eq!(games % 2, 0, "Number of games must be even");
        let mut results = [0; 3];
        let mut first_player_wins = 0;
        let mut second_player_wins = 0;
        for i in 0..games / 2 {
            print!(" Game {}/{games}    \r", i + 1);
            std::io::stdout().flush().unwrap();
            let result = Self::do_encounter(&mut self.players, false);
            match result {
                1 => results[0] += 1,  // X wins, so the first player wins
                0 => results[1] += 1,  // Draw, so no one wins
                -1 => results[2] += 1, // O wins, so the second player wins
                _ => panic!("Invalid result"),
            }
            match result {
                1 => first_player_wins += 1,
                -1 => second_player_wins += 1,
                _ => (),
            }
        }
        let first_half = results;
        for i in games / 2..games {
            print!(" Game {}/{games}    \r", i + 1);
            std::io::stdout().flush().unwrap();
            let result = Self::do_encounter(&mut self.players, true);
            match result {
                1 => results[2] += 1,  // X wins, so the second player wins
                0 => results[1] += 1,  // Draw, so no one wins
                -1 => results[0] += 1, // O wins, so the first player wins
                _ => panic!("Invalid result"),
            }
            match result {
                1 => first_player_wins += 1,
                -1 => second_player_wins += 1,
                _ => (),
            }
        }
        let second_half = [
            results[0] - first_half[0],
            results[1] - first_half[1],
            results[2] - first_half[2],
        ];
        println!(" Game {games}/{games}    ");
        println!("{RESET}");
        println!(
            "wins: {GREEN}{}{RESET}, draws: {}, losses: {RED}{}{RESET}",
            results[0], results[1], results[2]
        );
        println!(
            "wins: {GREEN}{}{RESET}, draws: {}, losses: {RED}{}{RESET} (first half)",
            first_half[0], first_half[1], first_half[2]
        );
        println!(
            "wins: {GREEN}{}{RESET}, draws: {}, losses: {RED}{}{RESET} (second half)",
            second_half[0], second_half[1], second_half[2]
        );
        println!(
            "going first resulted in {GREEN}{first_player_wins}{RESET} wins, {RED}{second_player_wins}{RESET} losses"
        );
        let elo = elo::difference(results[0], results[2], results[1]);
        let control = if elo.difference > 0.0 { GREEN } else { RED };
        println!(
            "Elo difference: {control}{:+.1}{RESET}, error: ±{:.1}",
            elo.difference, elo.error
        );
        println!(
            "Test results significant? {}",
            if elo.difference.abs() < elo.error {
                format!("{RED}NO{RESET}")
            } else {
                format!("{GREEN}YES{RESET}")
            }
        );
    }
}
