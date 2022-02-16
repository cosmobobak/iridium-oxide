use crate::{
    agent::Agent,
    game::{Game, MoveBuffer},
    mcts::MonteCarloTreeSearcher,
};

pub enum Player<G: Game> {
    Human,
    Computer(MonteCarloTreeSearcher<G>),
}

impl<G: Game> Agent<G> for Player<G> {
    fn transition(&mut self, state: G) -> G {
        let mut state = state;
        match self {
            Player::Human => {
                let mut buffer = G::Buffer::default();
                state.generate_moves(&mut buffer);
                println!("Your options are:");
                print!("[");
                for &m in buffer.iter().take(buffer.len() - 1) {
                    print!("{}, ", m);
                }
                println!("{}]", buffer.iter().last().unwrap());
                println!("Enter move: ");
                let mut user_input = String::new();
                std::io::stdin().read_line(&mut user_input).unwrap();
                let user_input = user_input.trim();
                let user_move = *buffer
                    .iter()
                    .find(|&&m| format!("{}", m) == user_input)
                    .expect("Invalid move");
                state.push(user_move);
                state
            }
            Player::Computer(agent) => agent.best_next_board(state),
        }
    }
}

pub struct GameRunner<G: Game> {
    state: G,
    players: [Player<G>; 2],
}

impl<G: Game> GameRunner<G> {
    pub fn new(state: G, player1: Player<G>, player2: Player<G>) -> Self {
        Self {
            state,
            players: [player1, player2],
        }
    }

    fn do_printout(&self) -> bool {
        self.players.iter().any(|p| matches!(p, Player::Human))
    }

    pub fn run(&mut self) {
        while !self.state.is_terminal() {
            if self.do_printout() {
                println!("{}", self.state);
            }
            let player = match self.state.turn() {
                1 => &mut self.players[0],
                -1 => &mut self.players[1],
                _ => panic!("Invalid turn"),
            };
            self.state = player.transition(self.state);
        }
        if self.do_printout() {
            println!("{}", self.state);
            println!("{}", self.state.outcome().unwrap());
        }
    }
}
