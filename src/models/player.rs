const TRAVERSER_BYTE: u8 = 0b10000000;
const OPPONENT_BYTE: u8 = 0b01000000;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Player {
    Traverser,
    Opponent,
}

impl Player {
    pub fn serialise(&self) -> u8 {
        match self {
            Player::Traverser => TRAVERSER_BYTE,
            Player::Opponent => OPPONENT_BYTE,
        }
    }

    pub fn deserialise(byte: &u8) -> Player {
        match *byte {
            TRAVERSER_BYTE => Player::Traverser,
            OPPONENT_BYTE => Player::Opponent,
            _ => panic!("Invalid byte for player deserialisation"),
        }
    }

    pub fn is_traverser(&self) -> bool {
        match self {
            Player::Traverser => true,
            _ => false,
        }
    }

    pub fn is_opponent(self) -> bool {
        match self {
            Player::Opponent => true,
            _ => false,
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
        assert_eq!(Player::deserialise(&TRAVERSER_BYTE), Player::Traverser);
        assert_eq!(Player::deserialise(&OPPONENT_BYTE), Player::Opponent);
    }
}
