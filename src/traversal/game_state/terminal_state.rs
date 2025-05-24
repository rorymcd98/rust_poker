/// Describes the ways in which a game or street can terminate, or None if the game is not terminal
#[derive(Debug)]
pub enum TerminalState {
    Showdown,
    Fold,
    StreetOver,
    None,
}