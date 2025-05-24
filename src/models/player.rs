use std::fmt::Display;
const TRAVERSER_BYTE: u8 = 0b10000000;
const OPPONENT_BYTE: u8 = 0b01000000;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum Player {
    #[default]
    Traverser,
    Opponent,
}

impl Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Player::Traverser => write!(f, "Tra"),
            Player::Opponent => write!(f, "Opp"),
        }
    }
}

impl Player {
    #[allow(dead_code)]
    pub fn serialise(&self) -> u8 {
        match self {
            Player::Traverser => TRAVERSER_BYTE,
            Player::Opponent => OPPONENT_BYTE,
        }
    }

    #[allow(dead_code)]
    pub fn deserialise(byte: &u8) -> Player {
        match *byte {
            TRAVERSER_BYTE => Player::Traverser,
            OPPONENT_BYTE => Player::Opponent,
            _ => panic!("Invalid byte for player deserialisation"),
        }
    }

    pub fn get_opposite(&self) -> Player {
        match self {
            Player::Traverser => Player::Opponent,
            Player::Opponent => Player::Traverser,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialise_player() {
        assert_eq!(Player::Traverser.serialise(), TRAVERSER_BYTE);
        assert_eq!(Player::Opponent.serialise(), OPPONENT_BYTE);
    }

    #[test]
    fn test_deserialise_player() {
        let traverser_matches = matches!(Player::deserialise(&TRAVERSER_BYTE), Player::Traverser);
        assert!(traverser_matches);
        let opponent_matches = matches!(Player::deserialise(&OPPONENT_BYTE), Player::Opponent);
        assert!(opponent_matches);
    }
}
