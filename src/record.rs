use std::io;

/// A data structure that records data from self-play.
#[derive(Debug, PartialEq)]
struct GameRecord {
    /// The header of the game record.
    header: GameRecordHeader,
    /// The entries of the game record.
    entries: Vec<GameRecordEntry>,
}

/// A header for a game record.
#[derive(Debug, PartialEq)]
struct GameRecordHeader {
    /// The number of moves in the game.
    move_count: u32,
    /// The outcome of the game.
    outcome: i8,
    /// The dimensions of action space.
    action_space_dimensions: Vec<usize>,
}

/// A single entry in a game record.
#[derive(Debug, PartialEq)]
struct GameRecordEntry {
    /// The number of rollouts assigned to each move.
    policy: Vec<u16>,
    /// The move played in the game.
    chosen_move: u32,
    /// The evaluation of the state.
    evaluation: f64,
}

impl GameRecord {
    /// Creates a new game record.
    fn new(header: GameRecordHeader) -> Self {
        Self {
            header,
            entries: Vec::new(),
        }
    }

    /// Adds a new entry to the game record.
    fn add_entry(&mut self, entry: GameRecordEntry) {
        self.entries.push(entry);
    }

    /// Writes the game record as bytes into the given `io::Write`.
    fn write_to<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        let policy_dim = self.header.action_space_dimensions.iter().product();
        writer.write_all(&self.header.move_count.to_le_bytes())?;
        writer.write_all(&self.header.outcome.to_le_bytes())?;
        writer.write_all(
            &TryInto::<u32>::try_into(self.header.action_space_dimensions.len())
                .unwrap()
                .to_le_bytes(),
        )?;
        for &dim in &self.header.action_space_dimensions {
            writer.write_all(&TryInto::<u32>::try_into(dim).unwrap().to_le_bytes())?;
        }
        writer.write_all(
            &TryInto::<u32>::try_into(self.entries.len())
                .unwrap()
                .to_le_bytes(),
        )?;
        for entry in &self.entries {
            assert_eq!(entry.policy.len(), policy_dim);
            writer.write_all(
                &TryInto::<u32>::try_into(entry.chosen_move)
                    .unwrap()
                    .to_le_bytes(),
            )?;
            writer.write_all(&entry.evaluation.to_le_bytes())?;
            for &p in &entry.policy {
                writer.write_all(&p.to_le_bytes())?;
            }
        }
        Ok(())
    }

    /// Reads a game record from the given `io::Read`.
    fn read_from<R: io::Read>(mut reader: R) -> io::Result<Self> {
        let mut move_count_bytes = [0u8; 4];
        reader.read_exact(&mut move_count_bytes)?;
        let move_count = u32::from_le_bytes(move_count_bytes);

        let mut outcome_bytes = [0u8; 1];
        reader.read_exact(&mut outcome_bytes)?;
        let outcome = i8::from_le_bytes(outcome_bytes);

        let mut action_space_dimensions_count_bytes = [0u8; 4];
        reader.read_exact(&mut action_space_dimensions_count_bytes)?;
        let action_space_dimensions_count = u32::from_le_bytes(action_space_dimensions_count_bytes);

        let mut action_space_dimensions = Vec::new();
        for _ in 0..action_space_dimensions_count {
            let mut dim_bytes = [0u8; 4];
            reader.read_exact(&mut dim_bytes)?;
            action_space_dimensions.push(u32::from_le_bytes(dim_bytes));
        }
        let policy_dim = action_space_dimensions.iter().product();

        let mut entries_count_bytes = [0u8; 4];
        reader.read_exact(&mut entries_count_bytes)?;
        let entries_count = u32::from_le_bytes(entries_count_bytes);

        let mut entries = Vec::with_capacity(entries_count as usize);
        for _ in 0..entries_count {
            let mut chosen_move_bytes = [0u8; 4];
            reader.read_exact(&mut chosen_move_bytes)?;
            let chosen_move = u32::from_le_bytes(chosen_move_bytes);
            let mut evaluation_bytes = [0u8; 8];
            reader.read_exact(&mut evaluation_bytes)?;
            let evaluation = f64::from_le_bytes(evaluation_bytes);

            let mut policy = Vec::new();
            for _ in 0..policy_dim {
                let mut p_bytes = [0u8; 2];
                reader.read_exact(&mut p_bytes)?;
                policy.push(u16::from_le_bytes(p_bytes));
            }

            entries.push(GameRecordEntry {
                policy,
                chosen_move,
                evaluation,
            });
        }

        Ok(Self {
            header: GameRecordHeader {
                move_count,
                outcome,
                action_space_dimensions: action_space_dimensions
                    .into_iter()
                    .map(|d| d as usize)
                    .collect(),
            },
            entries,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_record() {
        let mut record = GameRecord::new(GameRecordHeader {
            move_count: 3,
            outcome: 1,
            action_space_dimensions: vec![2, 3],
        });
        record.add_entry(GameRecordEntry {
            policy: vec![1, 2, 3, 4, 5, 6],
            chosen_move: 1,
            evaluation: 0.5,
        });
        record.add_entry(GameRecordEntry {
            policy: vec![7, 8, 9, 10, 11, 12],
            chosen_move: 2,
            evaluation: 0.75,
        });
        record.add_entry(GameRecordEntry {
            policy: vec![13, 14, 15, 16, 17, 18],
            chosen_move: 3,
            evaluation: 0.25,
        });
        let mut bytes = Vec::new();
        record.write_to(&mut bytes).unwrap();
        let record2 = GameRecord::read_from(&bytes[..]).unwrap();
        assert_eq!(record.header.move_count, record2.header.move_count);
        assert_eq!(record.header.outcome, record2.header.outcome);
        assert_eq!(
            record.header.action_space_dimensions,
            record2.header.action_space_dimensions
        );
        assert_eq!(record.entries, record2.entries);
    }

    #[test]
    fn game_record_fuzz() {
        let rng = fastrand::Rng::new();
        let mut buf = Vec::new();
        for _ in 0..100_000 {
            let mut record = GameRecord::new(GameRecordHeader {
                move_count: rng.u32(..),
                outcome: rng.i8(..),
                action_space_dimensions: (0..rng.u32(1..=3)).map(|_| rng.usize(1..=3)).collect(),
            });
            for _ in 0..rng.u32(..5) {
                record.add_entry(GameRecordEntry {
                    policy: (0..record.header.action_space_dimensions.iter().product())
                        .map(|_| rng.u16(..))
                        .collect(),
                    chosen_move: rng.u32(..),
                    evaluation: rng.f64(),
                });
            }
            buf.clear();
            record.write_to(&mut buf).unwrap();
            let record2 = GameRecord::read_from(&buf[..]).unwrap();
            assert_eq!(record, record2);
        }
    }
}
