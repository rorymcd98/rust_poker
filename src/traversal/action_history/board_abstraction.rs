use std::fmt::Display;

use crate::models::card::Card;
use crate::{evaluate::generate_tables::remaining_hand_types::HandType, models::card::Rank};

#[derive(Default)]
pub struct BoardAbstraction {
    pub max_consecutive_cards: u8,
    pub suit_count_abstraction: u8, // 0 effectively means rainbow, otherwise corresponds to the number of cards
    pub board_hand_type: HandType,
}

impl Display for BoardAbstraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Consecutive: {}, Suits: {}, Hand type: {:?}",
            self.max_consecutive_cards, self.suit_count_abstraction, self.board_hand_type
        )
    }
}

impl BoardAbstraction {
    pub fn new(cards: &[Card]) -> BoardAbstraction {
        let mut suits = [0; 4];
        let mut rank_counter = [0; 14];

        for c in cards {
            suits[c.suit.to_int() as usize] += 1;
            let rank_idx = c.rank.to_int() as usize;
            rank_counter[rank_idx + 1] += 1;
            if rank_idx == 12 {
                rank_counter[0] += 1;
            }
        }

        let mut connected_cards = [0; 3]; // Pair, trip, quad

        let mut consective = 0;
        let mut max_consecutive_cards = 0;
        for i in 0..14 {
            if rank_counter[i] > 1 && i > 0 {
                connected_cards[rank_counter[i] - 2] += 1;
            }

            if rank_counter[i] > 0 {
                consective += 1;
                max_consecutive_cards = std::cmp::max(max_consecutive_cards, consective);
            } else {
                consective = 0;
            }
        }

        let hand_type = match connected_cards {
            [1, 0, 0] => HandType::Pair(Rank::default()),
            [2, 0, 0] => HandType::TwoPair(Rank::default(), Rank::default()),
            [0, 1, 0] => HandType::ThreeOfAKind(Rank::default()),
            [1, 1, 0] => HandType::FullHouse(Rank::default(), Rank::default()),
            [0, 0, 1] => HandType::FourOfAKind(Rank::default()),
            _ => HandType::None,
        };

        let max_suits = *suits.iter().max().unwrap();

        // If it's 1 on the flop, 1 on the turn, 2 on the river, we just consider it 0
        let suit_count_abstraction = match (max_suits, cards.len()) {
            (1, 3) => 0,
            (1, 4) => 0,
            (2, 5) => 0,
            _ => max_suits,
        };

        BoardAbstraction {
            max_consecutive_cards,
            suit_count_abstraction,
            board_hand_type: hand_type,
        }
    }

    pub fn serialise(&self) -> Vec<u8> {
        let mut serialised = vec![];
        serialised.push(self.max_consecutive_cards);
        serialised.push(self.suit_count_abstraction);
        let hand_type_serialised = match self.board_hand_type {
            HandType::Pair(_) => 1,
            HandType::TwoPair(_, _) => 2,
            HandType::ThreeOfAKind(_) => 3,
            HandType::FullHouse(_, _) => 4,
            HandType::FourOfAKind(_) => 5,
            HandType::None => 0,
        };
        serialised.push(hand_type_serialised);
        serialised
    }

    pub fn deserialise(serialised: &[u8]) -> BoardAbstraction {
        BoardAbstraction {
            max_consecutive_cards: serialised[0],
            suit_count_abstraction: serialised[1],
            board_hand_type: match serialised[2] {
                1 => HandType::Pair(Rank::default()),
                2 => HandType::TwoPair(Rank::default(), Rank::default()),
                3 => HandType::ThreeOfAKind(Rank::default()),
                4 => HandType::FullHouse(Rank::default(), Rank::default()),
                5 => HandType::FourOfAKind(Rank::default()),
                _ => HandType::None,
            },
        }
    }

}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::card::{Card, Rank, Suit};

    #[test]
    fn test_board_abstraction_pair() {
        let cards = vec![
            Card::new(Suit::Hearts, Rank::Two),
            Card::new(Suit::Diamonds, Rank::Two),
            Card::new(Suit::Clubs, Rank::Three),
            Card::new(Suit::Spades, Rank::Four),
            Card::new(Suit::Hearts, Rank::Five),
        ];
        let board_abstraction = BoardAbstraction::new(&cards);
        assert_eq!(board_abstraction.max_consecutive_cards, 4);
        assert_eq!(board_abstraction.suit_count_abstraction, 0);
        assert!(matches!(
            board_abstraction.board_hand_type,
            HandType::Pair(_)
        ));
    }

    #[test]
    fn test_board_abstraction_two_pair() {
        let cards = vec![
            Card::new(Suit::Hearts, Rank::Two),
            Card::new(Suit::Diamonds, Rank::Two),
            Card::new(Suit::Clubs, Rank::Three),
            Card::new(Suit::Spades, Rank::Three),
            Card::new(Suit::Hearts, Rank::Five),
        ];
        let board_abstraction = BoardAbstraction::new(&cards);
        assert_eq!(board_abstraction.max_consecutive_cards, 2);
        assert_eq!(board_abstraction.suit_count_abstraction, 0);
        assert!(matches!(
            board_abstraction.board_hand_type,
            HandType::TwoPair(_, _)
        ));
    }

    #[test]
    fn test_board_abstraction_three_of_a_kind() {
        let cards = vec![
            Card::new(Suit::Hearts, Rank::Two),
            Card::new(Suit::Hearts, Rank::Two),
            Card::new(Suit::Hearts, Rank::Two),
            Card::new(Suit::Spades, Rank::Four),
            Card::new(Suit::Hearts, Rank::Five),
        ];
        let board_abstraction = BoardAbstraction::new(&cards);
        assert_eq!(board_abstraction.max_consecutive_cards, 2);
        assert_eq!(board_abstraction.suit_count_abstraction, 4);
        assert!(matches!(
            board_abstraction.board_hand_type,
            HandType::ThreeOfAKind(_)
        ));
    }

    #[test]
    fn test_board_abstraction_full_house() {
        let cards = vec![
            Card::new(Suit::Hearts, Rank::Two),
            Card::new(Suit::Diamonds, Rank::Two),
            Card::new(Suit::Clubs, Rank::Three),
            Card::new(Suit::Spades, Rank::Three),
            Card::new(Suit::Hearts, Rank::Three),
        ];
        let board_abstraction = BoardAbstraction::new(&cards);
        assert_eq!(board_abstraction.max_consecutive_cards, 2);
        assert_eq!(board_abstraction.suit_count_abstraction, 0);
        assert!(matches!(
            board_abstraction.board_hand_type,
            HandType::FullHouse(_, _)
        ));
    }

    #[test]
    fn test_board_abstraction_four_of_a_kind() {
        let cards = vec![
            Card::new(Suit::Hearts, Rank::Two),
            Card::new(Suit::Diamonds, Rank::Two),
            Card::new(Suit::Clubs, Rank::Two),
            Card::new(Suit::Spades, Rank::Two),
            Card::new(Suit::Hearts, Rank::Five),
        ];
        let board_abstraction = BoardAbstraction::new(&cards);
        assert_eq!(board_abstraction.max_consecutive_cards, 1);
        assert_eq!(board_abstraction.suit_count_abstraction, 1);
        assert!(matches!(
            board_abstraction.board_hand_type,
            HandType::FourOfAKind(_)
        ));
    }

    #[test]
    fn test_board_abstraction_none() {
        let cards = vec![
            Card::new(Suit::Hearts, Rank::Two),
            Card::new(Suit::Diamonds, Rank::Four),
            Card::new(Suit::Clubs, Rank::Six),
            Card::new(Suit::Spades, Rank::Eight),
            Card::new(Suit::Hearts, Rank::Ten),
        ];
        let board_abstraction = BoardAbstraction::new(&cards);
        assert_eq!(board_abstraction.max_consecutive_cards, 1);
        assert_eq!(board_abstraction.suit_count_abstraction, 0);
        assert!(matches!(board_abstraction.board_hand_type, HandType::None));
    }
}
