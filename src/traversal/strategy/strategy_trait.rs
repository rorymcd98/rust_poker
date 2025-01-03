use crate::traversal::action_history::action::DEFAULT_ACTION_COUNT;

pub trait Strategy {
    fn new(actions: usize) -> Self;
    fn get_current_strategy(&self, iteration: usize) -> [f32; DEFAULT_ACTION_COUNT];
    fn from_existing_strategy(actions: usize, strategy: [f32; DEFAULT_ACTION_COUNT]) -> Self;
    fn get_actions(&self) -> usize;
}