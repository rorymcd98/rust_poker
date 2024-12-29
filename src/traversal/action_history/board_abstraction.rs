use crate::{evaluate::generate_tables::remaining_hand_types::HandType, models::card::Rank, Card};
pub struct BoardAbstraction {
    pub max_consecutive_cards: u8,
    pub max_suit_count: u8,
    pub board_hand_type: HandType,
}

impl BoardAbstraction {
    pub fn new(cards: &[Card]) -> BoardAbstraction {
        let mut suits = [0; 4];
        let mut rank_counter = [0; 13];

        for c in cards {
            suits[c.suit.to_int() as usize] += 1;
            rank_counter[c.rank.to_int() as usize] += 1;
        }

        let mut connected_cards = [0; 3]; // Pair, trip, quad

        let mut consective = 0;
        let mut max_consecutive_cards = 0;
        for i in 0..13 {
            if rank_counter[i] > 1 {
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
            [1,0,0] => HandType::Pair(Rank::default()),
            [2,0,0] => HandType::TwoPair(Rank::default(), Rank::default()),
            [0,1,0] => HandType::ThreeOfAKind(Rank::default()),
            [1,1,0] => HandType::FullHouse(Rank::default(), Rank::default()),
            [0,0,1] => HandType::FourOfAKind(Rank::default()),
            _ => HandType::None,
        };

        BoardAbstraction {
            max_consecutive_cards,
            max_suit_count : *suits.iter().max().unwrap(),
            board_hand_type: hand_type,
        }
    }
}