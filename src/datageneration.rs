use std::{
    fmt::{self, Debug, Display, Formatter},
    fs::File,
    ops::Add,
};

use crate::{
    game::{Game, MoveBuffer},
    gamerunner::GameRunner,
    mcts::{self, Behaviour, SearchResults, MCTS, Limit},
};

/// A bitvector representation of a single game state.
pub struct StateVector {
    pub data: Vec<u8>,
}

/// A probability distribution over the possible moves in a single game state.
pub struct PolicyVector {
    pub data: Vec<f64>,
}

pub struct Entry {
    pub outcome: i8,
    pub move_count: u32,
    pub state: StateVector,
    pub policy: PolicyVector,
}

impl Display for Entry {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},", self.outcome, self.move_count)?;
        for &i in &self.state.data {
            write!(f, "{i},")?;
        }
        for &i in &self.policy.data[..self.policy.data.len() - 1] {
            write!(f, "{i:.3},")?;
        }
        let last = *self
            .policy
            .data
            .last()
            .expect("expected nonempty policy vector");
        write!(f, "{last:.3}")
    }
}

impl Debug for Entry {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "outcome: {}, move_count: {}, \nstate: \n {:?}, \npolicy: \n {:?}",
            self.outcome, self.move_count, self.state.data, self.policy.data
        )
    }
}

/// Display-wrapper for Entry that prints the input for prediction.
struct X<'a>(&'a Entry);

impl Display for X<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let l = self.0.state.data.len();
        for (i, v) in self.0.state.data.iter().enumerate() {
            write!(f, "{v}")?;
            if i + 1 != l {
                write!(f, ",")?;
            }
        }
        Ok(())
    }
}

/// Display-wrapper for Entry that prints the output for prediction.
struct Y<'a>(&'a Entry);

impl Display for Y<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let l = self.0.policy.data.len();
        for (i, v) in self.0.policy.data.iter().enumerate() {
            write!(f, "{v:.3}")?;
            if i + 1 != l {
                write!(f, ",")?;
            }
        }
        Ok(())
    }
}

pub trait VectoriseState: Game {
    fn csv_header() -> String;
    fn vectorise_state(&self) -> StateVector;
    fn index_move(m: Self::Move) -> usize;
    fn action_space() -> usize;
    fn state_vector_dimensions() -> Vec<usize>;

    fn policy_vector(&self, policy: &[f64]) -> PolicyVector {
        let mut out = vec![0.0; Self::action_space()];
        let mut buf = Self::Buffer::default();
        self.generate_moves(&mut buf);
        assert_eq!(policy.len(), buf.len());
        for (i, &m) in buf.iter().enumerate() {
            let index = Self::index_move(m);
            out[index] = policy[i];
        }
        PolicyVector { data: out }
    }
}

// TODO: This should be refactored to store a vector of episodes, each storing the single outcome of the episode.
//       This would allow for more efficient storage of data. Doing "slices" of a game into each state is weird.
//       You can argue that this makes it easy to shuffle the data later for training, but that should probably
//       be done in training and not generation or saving.
pub struct GameData {
    pub entries: Vec<Entry>,
    pub state_dimensions: Vec<usize>,
    pub action_space: usize,
}

impl GameData {
    pub fn save<G: VectoriseState>(&self, id: &str) -> Result<(), std::io::Error> {
        use std::io::Write;
        std::fs::create_dir_all("datasets")?;
        let mut file_x = File::create(format!("datasets/{id}-x.txt"))?;
        let mut file_y = File::create(format!("datasets/{id}-y.txt"))?;
        for entry in &self.entries {
            writeln!(file_x, "{}", X(entry))?;
            writeln!(file_y, "{}", Y(entry))?;
        }
        Ok(())
    }

    pub fn summary(&self) {
        #[allow(clippy::cast_precision_loss)]
        let len_float = self.entries.len() as f64;
        println!(
            "Mean game outcome: {:.3}",
            f64::from(
                self.entries
                    .iter()
                    .map(|e| i32::from(e.outcome))
                    .sum::<i32>()
            ) / len_float
        );
    }
}

impl<'a, G: VectoriseState + mcts::MCTSExt> GameRunner<'a, G> {
    pub fn play_training_game(flags: &Behaviour) -> GameData {
        let mut state = G::default();
        let mut states = Vec::new();
        let mut policies = Vec::new();
        let mut engine = MCTS::new(flags);
        while !state.is_terminal() {
            // clear the terminal
            // print!("\x1B[2J\x1B[1;1H");
            // println!("{state}");
            // println!("Thinking...");
            
            let s = state;
            let SearchResults {
                rollout_distribution,
                new_node,
                new_node_idx: _,
                rollouts,
                win_rate: _,
            } = engine.search(&s);
            assert!(match flags.limit {
                Limit::Time(_) => true,
                Limit::Rollouts(x) => x == rollouts,
            });
            let legal_policy = rollout_distribution
                .into_iter()
                .map(|rs| f64::from(rs) / f64::from(rollouts))
                .collect::<Vec<_>>();
            let policy = s.policy_vector(&legal_policy);
            states.push(s.vectorise_state());
            policies.push(policy);
            state = new_node;
        }
        let outcome = state.evaluate();
        assert_eq!(states.len(), policies.len());
        #[allow(clippy::cast_possible_truncation)]
        let entries = states
            .into_iter()
            .zip(policies)
            .enumerate()
            .map(|(i, (s, p))| Entry {
                outcome,
                move_count: i as u32,
                state: s,
                policy: p,
            })
            .collect();
        GameData {
            entries,
            state_dimensions: G::state_vector_dimensions(),
            action_space: G::action_space(),
        }
    }
}

impl Add for GameData {
    type Output = Self;

    fn add(self, mut other: Self) -> Self::Output {
        assert_eq!(self.state_dimensions, other.state_dimensions);
        assert_eq!(self.action_space, other.action_space);
        let mut entries = self.entries;
        entries.append(&mut other.entries);
        Self {
            entries,
            state_dimensions: self.state_dimensions.clone(),
            action_space: self.action_space,
        }
    }
}

impl Display for GameData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for entry in &self.entries {
            writeln!(f, "{entry}")?;
        }
        Ok(())
    }
}

impl Debug for GameData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{self}")?;
        writeln!(f, "First five entries:")?;
        for entry in self.entries.iter().take(5) {
            writeln!(f, "{entry:?}")?;
        }
        Ok(())
    }
}
