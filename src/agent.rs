use crate::game::Game;

pub trait Agent<G: Game> {
    fn transition(&mut self, state: G) -> G;
}
