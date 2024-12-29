use std::fmt::Display;

use crate::models::card::Card;

pub const DEFAULT_ACTION_COUNT: usize = 3;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Action {
    CheckFold,
    Call,
    Bet, // if we want this to become no-limit this can take in a u32
    Deal(Card)
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::CheckFold => write!(f, "ChF"),
            Action::Call => write!(f, "Cal"),
            Action::Bet => write!(f, "Bet"),
            Action::Deal(card) => write!(f, "D({})", card),
        }
    }
}

impl Action {
    pub const CHECKFOLD_BYTE: u8 = 0b00000000;
    pub const CALL_BYTE: u8 = 0b01000000;
    pub const BET_BYTE: u8 = 0b10000000;
    pub const DEAL_BYTE: u8 = 0b11000000;
    pub const ACTION_MASK: u8 = 0b11000000;
    pub const CARD_MASK: u8 = 0b00111111;

    pub fn serialise(&self) -> u8 {
        match self {
            Action::CheckFold => Self::CHECKFOLD_BYTE,
            Action::Call => Self::CALL_BYTE,
            Action::Bet => Self::BET_BYTE,
            Action::Deal(card) => card.serialise() | Self::DEAL_BYTE,
        }
    }
    
    pub fn deserialise(byte: &u8) -> Action {
        match byte & Self::ACTION_MASK {
            Self::CHECKFOLD_BYTE => Action::CheckFold,
            Self::CALL_BYTE => Action::Call,
            Self::BET_BYTE => Action::Bet,
            _ => Action::Deal(Card::deserialise(*byte)),
        }
    }

    pub fn is_deal(&self) -> bool {
        match self {
            Action::Deal(_) => true,
            _ => false,
        }
    }

    pub fn is_call(&self) -> bool {
        match self {
            Action::Call => true,
            _ => false,
        }
    }

    pub fn is_bet(&self) -> bool {
        match self {
            Action::Bet => true,
            _ => false,
        }
    }

    pub fn is_checkfold(&self) -> bool {
        match self {
            Action::CheckFold => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::card::{Card, Suit, Rank};
    use rstest::rstest;

    const TEST_CARD: Card = Card { suit: Suit::Hearts, rank: Rank::Two };

    #[test]
    fn test_round_trip_checkfold() {
        let action = Action::CheckFold;
        let byte = action.serialise();
        assert_eq!(Action::deserialise(&byte), action);
    }

    #[test]
    fn test_round_trip_call() {
        let action = Action::Call;
        let byte = action.serialise();
        assert_eq!(Action::deserialise(&byte), action);
    }

    #[test]
    fn test_round_trip_bet() {
        let action = Action::Bet;
        let byte = action.serialise();
        assert_eq!(Action::deserialise(&byte), action);
    }

    #[rstest]
    #[case(Card::new(Suit::Spades, Rank::Two))]
    #[case(Card::new(Suit::Hearts, Rank::Three))]
    #[case(Card::new(Suit::Diamonds, Rank::Four))]
    #[case(Card::new(Suit::Clubs, Rank::Five))]
    #[case(Card::new(Suit::Spades, Rank::Six))]
    #[case(Card::new(Suit::Hearts, Rank::Seven))]
    #[case(Card::new(Suit::Diamonds, Rank::Eight))]
    #[case(Card::new(Suit::Clubs, Rank::Nine))]
    #[case(Card::new(Suit::Spades, Rank::Ten))]
    #[case(Card::new(Suit::Hearts, Rank::Jack))]
    #[case(Card::new(Suit::Diamonds, Rank::Queen))]
    #[case(Card::new(Suit::Clubs, Rank::King))]
    #[case(Card::new(Suit::Spades, Rank::Ace))]
    #[case(Card::new(Suit::Hearts, Rank::Two))]
    #[case(Card::new(Suit::Diamonds, Rank::Three))]
    #[case(Card::new(Suit::Clubs, Rank::Four))]
    fn test_round_trip_deal(#[case] card: Card) {
        let action = Action::Deal(card);
        let byte = action.serialise();
        assert_eq!(Action::deserialise(&byte), action);
    }


    #[test]
    fn test_is_deal() {
        let card = TEST_CARD;
        let action = Action::Deal(card);
        assert!(action.is_deal());
        assert!(!Action::CheckFold.is_deal());
        assert!(!Action::Call.is_deal());
        assert!(!Action::Bet.is_deal());
    }

    #[test]
    fn test_is_call() {
        let action = Action::Call;
        assert!(action.is_call());
        assert!(!Action::CheckFold.is_call());
        assert!(!Action::Deal(TEST_CARD).is_call());
        assert!(!Action::Bet.is_call());
    }

    #[test]
    fn test_is_bet() {
        let action = Action::Bet;
        assert!(action.is_bet());
        assert!(!Action::CheckFold.is_bet());
        assert!(!Action::Call.is_bet());
        assert!(!Action::Deal(TEST_CARD).is_bet());
    }
}