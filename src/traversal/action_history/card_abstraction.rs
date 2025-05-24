use crate::models::card::{Card, Rank};
use std::fmt::Display;

///  Bucket ints to A-K, Q-9, 8-6, 5-2
fn bucket_rank(rank_int: usize) -> Rank {
    match rank_int {
        1..5 => Rank::Five,
        5..8 => Rank::Eight,
        8..11 => Rank::Queen,
        11..13 => Rank::Ace,
        _ => panic!("Invalid rank int {}", rank_int),
    }
}

/// Abstraction of a straight (part of the game abstraction)
pub struct StraightAbstraction {
    /// The highest card, but bucketed according to rank
    pub bucketed_high_card: Rank, // Can be further bucketed e.g. A-K, Q-T, 9-5
    /// The number of cards in the straight
    pub cards_in_straight: u8, // 0, 1, (& 2 on flop)
    /// Whether the straight requires a gutshot
    pub requires_gutshot: bool,
}

impl Display for StraightAbstraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}-high straight, {} cards, gutshot: {}",
            self.bucketed_high_card, self.cards_in_straight, self.requires_gutshot
        )
    }
}

impl StraightAbstraction {
    pub fn serialise(&self) -> u8 {
        let highest_card = self.bucketed_high_card.to_int(); // Because this will be at least 2, we should avoid parity with None
        let cards_in_straight = self.cards_in_straight;
        let requires_gutshot = self.requires_gutshot as u8;
        (highest_card << 4) | (cards_in_straight << 1) | requires_gutshot
    }

    pub fn deserialise(serialised: &u8) -> StraightAbstraction {
        let highest_card = Rank::from_int(serialised >> 4);
        let cards_in_straight = (serialised >> 1) & 0b111;
        let requires_gutshot = (serialised & 0b1) == 1;
        StraightAbstraction {
            bucketed_high_card: highest_card,
            cards_in_straight,
            requires_gutshot,
        }
    }
}

// TODO - method needs refactoring!
/// Find the highest straight that is either length 4 with a gutshot, or length 3-5 without a gutshot
pub fn get_straight_abstraction(
    hole_cards: &[Card; 2],
    board_cards: &[Card],
) -> Option<StraightAbstraction> {
    if board_cards.is_empty() {
        return None;
    }
    let mut rank_counts = [false; 14];
    for card in board_cards {
        let idx = (card.rank.to_int() + 1) as usize;
        rank_counts[idx] = true;
        if idx == 14 {
            rank_counts[0] = true; // low ace
        }
    }

    let mut candidate_hole_cards_indeces = Vec::with_capacity(2);
    for card in hole_cards {
        let rank_index = (card.rank.to_int() + 1) as usize;
        if !rank_counts[rank_index] {
            candidate_hole_cards_indeces.push(rank_index);
            rank_counts[rank_index] = true;
            if rank_index == 13 {
                rank_counts[0] = true; // low ace
            }
        }
    }

    if candidate_hole_cards_indeces.is_empty() {
        return None;
    }

    let mut highest_non_player_straight_card: u8 = 0;

    let mut longest_straight_without_gutshot = 0;
    let mut openended_high_card = 0;

    let mut longest_straight_with_gutshot = 0;
    let mut gutshot_high_card = 0;

    // roll a len 5 window over the ranks checking if it's a straight, and updating either len 4 gutshots, or len 3+ otherwise
    let mut in_window = 0;
    let mut consecutive = 0;

    for i in 0..4 {
        if i < 4 {
            if rank_counts[i] {
                in_window += 1;
                consecutive += 1;

                if candidate_hole_cards_indeces
                    .iter()
                    .any(|&x| x <= i && x >= i.saturating_sub(4))
                    && consecutive >= 3
                {
                    longest_straight_without_gutshot = consecutive;
                    openended_high_card = i;
                }
            } else {
                consecutive = 0;
            }
        }
    }

    for i in 4..14 {
        if rank_counts[i] {
            in_window += 1;
            consecutive += 1;
            if candidate_hole_cards_indeces
                .iter()
                .any(|&x| x <= i && x >= i - 4)
            {
                if in_window >= 3 && consecutive >= 3 {
                    longest_straight_without_gutshot = consecutive.min(5);
                    openended_high_card = i;
                } else if in_window == 4 && consecutive < 4 {
                    // for the case of abstraction we only care about gutshot 4
                    longest_straight_with_gutshot = 4;
                    gutshot_high_card = i;
                }
            } else if in_window == 5 {
                highest_non_player_straight_card = i as u8;
            }
        } else {
            consecutive = 0;
        }
        if rank_counts[i - 4] {
            in_window -= 1;
        }
    }

    if highest_non_player_straight_card > (openended_high_card.max(gutshot_high_card) as u8) {
        // If the highest straight is on the board, we don't care about the abstraction
        return None;
    }

    if 5 - longest_straight_with_gutshot.max(longest_straight_without_gutshot)
        > 5 - board_cards.len() as u8
    {
        // If there aren't enough cards left to make a straight,
        return None;
    }

    if longest_straight_with_gutshot > longest_straight_without_gutshot {
        // If the best gutshot is better than the best open-ended straight, use the gutshot
        Some(StraightAbstraction {
            bucketed_high_card: bucket_rank(gutshot_high_card - 1), // -1 due to low ace
            cards_in_straight: longest_straight_with_gutshot,
            requires_gutshot: true,
        })
    } else if longest_straight_without_gutshot > 0 {
        // Otherwise check if there is a straight without a gutshot
        return Some(StraightAbstraction {
            bucketed_high_card: bucket_rank(openended_high_card - 1),
            cards_in_straight: longest_straight_without_gutshot,
            requires_gutshot: false,
        });
    } else {
        // Otherwise there is no straight or straight draw
        return None;
    }
}

#[cfg(test)]
mod straight_abstraction_tests {
    use super::*;
    use crate::models::card::Suit;
    use rstest::rstest;

    #[test]
    fn test_bucket_rank() {
        assert_eq!(bucket_rank(2), Rank::Five);
        assert_eq!(bucket_rank(5), Rank::Eight);
        assert_eq!(bucket_rank(8), Rank::Queen);
        assert_eq!(bucket_rank(11), Rank::Ace);
    }

    #[rstest]
    #[case(0)]
    #[case(13)]
    #[case(14)]
    #[should_panic(expected = "Invalid rank int")]
    fn test_bucket_rank_invalid(#[case] rank_int: usize) {
        bucket_rank(rank_int);
    }

    #[rstest]
    #[case(2)]
    #[case(3)]
    #[case(4)]
    #[case(5)]
    #[case(6)]
    #[case(7)]
    #[case(8)]
    #[case(9)]
    #[case(10)]
    #[case(11)]
    #[case(12)]
    fn test_should_not_panic(#[case] rank_int: usize) {
        bucket_rank(rank_int);
    }

    #[test]
    fn test_get_straight_abstraction() {
        let hole_cards = [
            Card {
                rank: Rank::Ten,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Hearts,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Nine,
                suit: Suit::Clubs,
            },
            Card {
                rank: Rank::Eight,
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::Seven,
                suit: Suit::Spades,
            },
        ];
        let abstraction = get_straight_abstraction(&hole_cards, &board_cards).unwrap();
        assert_eq!(abstraction.bucketed_high_card, Rank::Queen); // rounded
        assert_eq!(abstraction.cards_in_straight, 5);
        assert!(!abstraction.requires_gutshot);
    }

    #[test]
    fn identifies_open_ended_ace_low_straight() {
        let hole_cards = [
            Card {
                rank: Rank::Ace,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Two,
                suit: Suit::Hearts,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Three,
                suit: Suit::Clubs,
            },
            Card {
                rank: Rank::Four,
                suit: Suit::Diamonds,
            },
        ];
        let abstraction = get_straight_abstraction(&hole_cards, &board_cards).unwrap();
        assert_eq!(abstraction.bucketed_high_card, Rank::Five);
        assert_eq!(abstraction.cards_in_straight, 4);
        assert!(!abstraction.requires_gutshot);
    }

    #[test]
    fn returns_none_for_2_draw_on_turn() {
        let hole_cards = [
            Card {
                rank: Rank::Ace,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Nine,
                suit: Suit::Hearts,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Three,
                suit: Suit::Clubs,
            },
            Card {
                rank: Rank::Two,
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::Ten,
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Diamonds,
            },
        ];
        let abstraction = get_straight_abstraction(&hole_cards, &board_cards);
        assert!(abstraction.is_none());
    }

    #[test]
    fn identifies_open_low_ace_straight() {
        let hole_cards = [
            Card {
                rank: Rank::Ace,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Two,
                suit: Suit::Hearts,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Three,
                suit: Suit::Clubs,
            },
            Card {
                rank: Rank::Four,
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::Five,
                suit: Suit::Diamonds,
            },
        ];
        let abstraction = get_straight_abstraction(&hole_cards, &board_cards).unwrap();
        assert_eq!(abstraction.bucketed_high_card, Rank::Five);
        assert_eq!(abstraction.cards_in_straight, 5);
        assert!(!abstraction.requires_gutshot);
    }

    #[test]
    fn identifies_nut_straight() {
        let hole_cards = [
            Card {
                rank: Rank::Ace,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Hearts,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Ten,
                suit: Suit::Clubs,
            },
            Card {
                rank: Rank::Queen,
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Diamonds,
            },
        ];
        let abstraction = get_straight_abstraction(&hole_cards, &board_cards).unwrap();
        assert_eq!(abstraction.bucketed_high_card, Rank::Ace);
        assert_eq!(abstraction.cards_in_straight, 5);
        assert!(!abstraction.requires_gutshot);
    }

    #[test]
    fn identifies_gut_shot_low_ace() {
        let hole_cards = [
            Card {
                rank: Rank::Ace,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Two,
                suit: Suit::Hearts,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Four,
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::Five,
                suit: Suit::Diamonds,
            },
        ];
        let abstraction = get_straight_abstraction(&hole_cards, &board_cards).unwrap();
        assert_eq!(abstraction.bucketed_high_card, Rank::Five);
        assert_eq!(abstraction.cards_in_straight, 4);
        assert!(abstraction.requires_gutshot);
    }

    #[test]
    fn test_get_straight_abstraction_with_gutshot() {
        let hole_cards = [
            Card {
                rank: Rank::Five,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Queen,
                suit: Suit::Hearts,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Ten,
                suit: Suit::Clubs,
            },
            Card {
                rank: Rank::Nine,
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::Eight,
                suit: Suit::Spades,
            },
            Card {
                rank: Rank::Six,
                suit: Suit::Hearts,
            },
        ];
        let abstraction = get_straight_abstraction(&hole_cards, &board_cards).unwrap();
        assert_eq!(abstraction.bucketed_high_card, Rank::Queen);
        assert_eq!(abstraction.cards_in_straight, 4);
        assert!(abstraction.requires_gutshot);
    }

    #[test]
    fn test_straight_is_on_the_board() {
        let hole_cards = [
            Card {
                rank: Rank::Two,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Three,
                suit: Suit::Hearts,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Seven,
                suit: Suit::Clubs,
            },
            Card {
                rank: Rank::Eight,
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::Nine,
                suit: Suit::Spades,
            },
            Card {
                rank: Rank::Ten,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Hearts,
            },
        ];
        let abstraction = get_straight_abstraction(&hole_cards, &board_cards);
        assert!(abstraction.is_none());
    }

    #[test]
    fn test_high_board_straight_overrules() {
        let hole_cards = [
            Card {
                rank: Rank::Two,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Six,
                suit: Suit::Hearts,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Seven,
                suit: Suit::Clubs,
            },
            Card {
                rank: Rank::Eight,
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::Nine,
                suit: Suit::Spades,
            },
            Card {
                rank: Rank::Ten,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Hearts,
            },
        ];
        let abstraction = get_straight_abstraction(&hole_cards, &board_cards);
        assert!(abstraction.is_none());
    }

    #[test]
    fn test_high_hand_straight_overrules() {
        let hole_cards = [
            Card {
                rank: Rank::Two,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Queen,
                suit: Suit::Hearts,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Seven,
                suit: Suit::Clubs,
            },
            Card {
                rank: Rank::Eight,
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::Nine,
                suit: Suit::Spades,
            },
            Card {
                rank: Rank::Ten,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Hearts,
            },
        ];
        let abstraction = get_straight_abstraction(&hole_cards, &board_cards)
            .expect("Expected abstraction to be generated");
        assert_eq!(abstraction.bucketed_high_card, Rank::Queen);
        assert_eq!(abstraction.cards_in_straight, 5);
        assert!(!abstraction.requires_gutshot);
    }
}

pub struct FlushAbstraction {
    pub flush_score: u8, // 0 == nut flush, 1 == second or third nut flush, 2 == fourth or worse flush
    pub cards_to_draw: u8, // 0, 1, (& 2 on flop)
}

impl Display for FlushAbstraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} flush, {} cards to draw",
            match self.flush_score {
                0 => "Nut",
                1 => "Second or third nut",
                2 => "Fourth or worse",
                _ => panic!("Invalid flush score {}", self.flush_score),
            },
            self.cards_to_draw
        )
    }
}

impl FlushAbstraction {
    pub fn serialise(&self) -> u8 {
        (self.flush_score << 2) | (self.cards_to_draw + 1) // +1 to avoid parity with None
    }

    pub fn deserialise(serialised: &u8) -> FlushAbstraction {
        FlushAbstraction {
            flush_score: (serialised >> 2) & 0b11,
            cards_to_draw: (serialised & 0b11) - 1,
        }
    }
}

// Check if the player has made a flush
// If they have score the player's flush based on the number of
// Mark the number of cards the player needs to draw
// If they haven't, return None
//
// Note: We don't check if the board flush dominates the player's flush, the player could still lose a showdown against a higher flush
// The score should give an indication of how much we can bluff in a split-pot board flush, or how much we should fear a higher flush
pub fn get_flush_abstraction(
    hole_cards: &[Card; 2],
    board_cards: &[Card],
) -> Option<FlushAbstraction> {
    if board_cards.is_empty() {
        return None;
    }

    let mut suit_counts = [0; 4];
    let mut player_has_suit = [false; 4];

    // Bucket the suits and say which are in the hand vs on the board
    for card in board_cards {
        let idx = card.suit.to_int() as usize;
        suit_counts[idx] += 1;
    }
    for card in hole_cards {
        let idx = card.suit.to_int() as usize;
        suit_counts[idx] += 1;
        player_has_suit[idx] = true;
    }

    // Initialise to the second hole card, that way if there's a tie it will be the player's high card that wins
    let mut most_flush_count_player = suit_counts[hole_cards[1].suit.to_int() as usize];
    let mut most_flush_suit_player = hole_cards[1].suit.to_int() as usize;

    for i in 0..4 {
        if player_has_suit[i] && suit_counts[i] > most_flush_count_player {
            most_flush_count_player = suit_counts[i];
            most_flush_suit_player = i;
        }
    }

    let cards_to_draw = 5 - most_flush_count_player.min(5);

    if cards_to_draw >= 3 || cards_to_draw > 5 - board_cards.len() {
        // If we have to draw three or more cards, or there aren't enough cards left to make a flush
        return None;
    }

    // Calculate how many missing cards beat the player's flush
    let matches_players_high_card = most_flush_suit_player == hole_cards[1].suit.to_int() as usize;
    let most_flushing_player_rank = if matches_players_high_card {
        hole_cards[1].rank.to_int()
    } else {
        hole_cards[0].rank.to_int()
    };

    let mut beating_player_card = 0;
    for card in board_cards {
        if card.suit.to_int() == most_flush_suit_player as u8
            && card.rank.to_int() > most_flushing_player_rank
        {
            beating_player_card += 1;
        }
    }

    // E.g. Ace (12) has 0 cards beating it, meaning 0 missing cards (12 - 12 - 0 == 0)
    // E.g. Jack (9) might have 2 board cards beating it (let's say KQ), meaning 1 missing card (the ace) (12 - 9 - 2 == 1)
    // E.g. 5 (3) might have 4 board cards beating it (let's say 6789), meaning 5 missing cards (T-A) (12 - 3 - 4 == 5)
    let missing_cards_that_beat_player = 12 - most_flushing_player_rank - beating_player_card;

    let player_flush_score = match missing_cards_that_beat_player {
        0 => 0,     // Nut flush
        1..=2 => 1, // Second or third nut flush
        _ => 2,     // Fourth or worse flush
    };

    if most_flush_count_player >= 3 {
        return Some(FlushAbstraction {
            flush_score: player_flush_score,
            cards_to_draw: cards_to_draw as u8,
        });
    }

    None
}

#[cfg(test)]
mod flush_abstraction_tests {
    use super::*;
    use crate::models::card::Suit;

    #[test]
    fn get_flush_low_score() {
        let hole_cards = [
            Card {
                rank: Rank::Ten,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Hearts,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Nine,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Eight,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Seven,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Six,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Five,
                suit: Suit::Hearts,
            },
        ];
        let abstraction = get_flush_abstraction(&hole_cards, &board_cards).unwrap();
        assert_eq!(abstraction.cards_to_draw, 0);
        assert_eq!(abstraction.flush_score, 2);
    }

    #[test]
    fn get_flush_mid_score() {
        let hole_cards = [
            Card {
                rank: Rank::Two,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Hearts,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Nine,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Eight,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Seven,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Six,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Five,
                suit: Suit::Hearts,
            },
        ];
        let abstraction = get_flush_abstraction(&hole_cards, &board_cards).unwrap();
        assert_eq!(abstraction.cards_to_draw, 0);
        assert_eq!(abstraction.flush_score, 1);
    }

    #[test]
    fn get_flush_high_score() {
        let hole_cards = [
            Card {
                rank: Rank::Ten,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Hearts,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Nine,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Eight,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Ace,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Queen,
                suit: Suit::Hearts,
            },
        ];
        let abstraction = get_flush_abstraction(&hole_cards, &board_cards).unwrap();
        assert_eq!(abstraction.cards_to_draw, 0);
        assert_eq!(abstraction.flush_score, 0);
    }
}

// The connected abstractions tell us how much our hand connects with the board
// This does not equate to the exact hand strength, as you may have (e.g.) a full house with a pair on the board, however this info should get encoded into the info set through the board state
pub enum ConnectedCardsAbstraction {
    Pair(PairAbstraction),
    TwoPair(TwoPairAbstraction),
    ThreeOfAKind(ThreeOfAKindAbstraction),
    FullHouse(FullHouseAbstraction),
    FourOfAKind, // Whether or not you have a pocket-pair should be enough to describe your infoset
}

impl Display for ConnectedCardsAbstraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectedCardsAbstraction::Pair(p) => write!(f, "Pair {}", p.pair_order_score),
            ConnectedCardsAbstraction::TwoPair(tp) => {
                write!(f, "Two Pair {}", tp.two_pair_order_score)
            }
            ConnectedCardsAbstraction::ThreeOfAKind(toak) => {
                write!(f, "Three of a Kind {}", toak.toak_order_score)
            }
            ConnectedCardsAbstraction::FullHouse(fh) => {
                write!(f, "Full House {}", fh.high_card_is_house)
            }
            ConnectedCardsAbstraction::FourOfAKind => write!(f, "Four of a Kind"),
        }
    }
}

impl ConnectedCardsAbstraction {
    pub fn serialise(&self) -> u8 {
        match self {
            ConnectedCardsAbstraction::Pair(p) => 1 << 4 | p.pair_order_score,
            ConnectedCardsAbstraction::TwoPair(tp) => 2 << 4 | tp.two_pair_order_score,
            ConnectedCardsAbstraction::ThreeOfAKind(toak) => 3 << 4 | toak.toak_order_score,
            ConnectedCardsAbstraction::FullHouse(fh) => 4 << 4 | fh.high_card_is_house as u8,
            ConnectedCardsAbstraction::FourOfAKind => 5,
        }
    }

    pub fn deserialise(serialised: &u8) -> ConnectedCardsAbstraction {
        match serialised >> 4 {
            1 => ConnectedCardsAbstraction::Pair(PairAbstraction {
                pair_order_score: serialised & 0b1111,
            }),
            2 => ConnectedCardsAbstraction::TwoPair(TwoPairAbstraction {
                two_pair_order_score: serialised & 0b1111,
            }),
            3 => ConnectedCardsAbstraction::ThreeOfAKind(ThreeOfAKindAbstraction {
                toak_order_score: serialised & 0b1111,
            }),
            4 => ConnectedCardsAbstraction::FullHouse(FullHouseAbstraction {
                high_card_is_house: (serialised & 0b1) == 1,
            }),
            5 => ConnectedCardsAbstraction::FourOfAKind,
            _ => panic!(
                "Invalid connected card abstraction serialisation {}",
                serialised
            ),
        }
    }
}

pub struct FullHouseAbstraction {
    pub high_card_is_house: bool,
}

pub struct PairAbstraction {
    pub pair_order_score: u8, // 0 = top pair, 1 = second pair, 2 = third or more pair
}

pub struct TwoPairAbstraction {
    pub two_pair_order_score: u8, // 0 = top two pair, 1 = top high pair, 2 = mid high pair
}

pub struct ThreeOfAKindAbstraction {
    pub toak_order_score: u8, // 0 = top toak, 1 = second toak, 2 = third or more toak
}

pub fn get_connected_card_abstraction(
    hole_cards: &[Card; 2],
    board_cards: &[Card],
) -> Option<ConnectedCardsAbstraction> {
    if board_cards.is_empty() {
        return None;
    }
    let mut rank_counts = [0u8; 13];
    let mut board_has_rank = [false; 13];
    for card in hole_cards {
        let idx = card.rank.to_int() as usize;
        rank_counts[idx] += 1;
    }
    for card in board_cards {
        let idx = card.rank.to_int() as usize;
        if rank_counts[idx] > 0 {
            rank_counts[idx] += 1;
        }
        board_has_rank[idx] = true;
    }

    let mut highest_pair_rank = 0;
    let mut highest_toak_rank = 0;
    let mut seen_so_far_pair = 30;
    let mut seen_so_far_toak = 30;
    let mut seen_so_far = 0;

    let mut counts = [0; 3];
    for i in (0..13).rev() {
        let rank_count = rank_counts[i];
        match rank_count {
            2 => {
                highest_pair_rank = highest_pair_rank.max(i + 1); // this +1 is to disambiguate 0
                seen_so_far_pair = seen_so_far_pair.min(seen_so_far);
            }
            3 => {
                highest_toak_rank = highest_toak_rank.max(i + 1);
                seen_so_far_toak = seen_so_far_toak.min(seen_so_far);
            }
            _ => {}
        }
        if rank_counts[i] > 1 {
            if rank_count == 5 {
                println!("Five of a kind! {:?}, {:?}", hole_cards, board_cards);
            }
            counts[rank_count as usize - 2] += 1;
        }
        if board_has_rank[i] {
            seen_so_far += 1;
        }
    }

    if counts[2] > 0 {
        Some(ConnectedCardsAbstraction::FourOfAKind)
    } else if counts[1] > 0 && counts[0] > 0 {
        return Some(ConnectedCardsAbstraction::FullHouse(FullHouseAbstraction {
            high_card_is_house: highest_pair_rank < highest_toak_rank,
        }));
    } else if counts[1] > 0 {
        return Some(ConnectedCardsAbstraction::ThreeOfAKind(
            ThreeOfAKindAbstraction {
                toak_order_score: seen_so_far_toak.min(2),
            },
        )); // Here we classify some Full houses as ToaKs, but it shouldn't matter since the board pairing will contain info about pairs and other ToakS
    } else if counts[0] > 1 {
        return Some(ConnectedCardsAbstraction::TwoPair(TwoPairAbstraction {
            two_pair_order_score: seen_so_far_pair.min(2),
        }));
    } else if counts[0] > 0 {
        return Some(ConnectedCardsAbstraction::Pair(PairAbstraction {
            pair_order_score: seen_so_far_pair.min(2),
        }));
    } else {
        return None;
    }
}

#[cfg(test)]
mod connected_cards_abstraction_tests {
    use crate::models::card::{Card, Rank, Suit};

    use super::{get_connected_card_abstraction, ConnectedCardsAbstraction};

    #[test]
    fn test_full_house_high_hosue() {
        let hole_cards = [
            Card {
                rank: Rank::Ten,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Nine,
                suit: Suit::Diamonds,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Ten,
                suit: Suit::Clubs,
            },
            Card {
                rank: Rank::Ten,
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::Nine,
                suit: Suit::Spades,
            },
        ];
        let abstraction = get_connected_card_abstraction(&hole_cards, &board_cards).unwrap();
        match abstraction {
            ConnectedCardsAbstraction::FullHouse(abstraction) => {
                assert!(abstraction.high_card_is_house);
            }
            _ => panic!("Expected FullHouse abstraction"),
        }
    }

    #[test]
    fn test_full_house_low_house() {
        let hole_cards = [
            Card {
                rank: Rank::Ten,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Nine,
                suit: Suit::Diamonds,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Ten,
                suit: Suit::Clubs,
            },
            Card {
                rank: Rank::Nine,
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::Nine,
                suit: Suit::Spades,
            },
        ];
        let abstraction = get_connected_card_abstraction(&hole_cards, &board_cards).unwrap();
        match abstraction {
            ConnectedCardsAbstraction::FullHouse(abstraction) => {
                assert!(!abstraction.high_card_is_house);
            }
            _ => panic!("Expected FullHouse abstraction"),
        }
    }

    #[test]
    fn test_get_connected_card_abstraction_four_of_a_kind() {
        let hole_cards = [
            Card {
                rank: Rank::Ten,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Ten,
                suit: Suit::Diamonds,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Ten,
                suit: Suit::Clubs,
            },
            Card {
                rank: Rank::Ten,
                suit: Suit::Spades,
            },
            Card {
                rank: Rank::Nine,
                suit: Suit::Diamonds,
            },
        ];
        let abstraction = get_connected_card_abstraction(&hole_cards, &board_cards).unwrap();
        match abstraction {
            ConnectedCardsAbstraction::FourOfAKind => {}
            _ => panic!("Expected FourOfAKind abstraction"),
        }
    }

    #[test]
    fn test_pair_abstraction() {
        let hole_cards = [
            Card {
                rank: Rank::Ten,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Nine,
                suit: Suit::Diamonds,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Ten,
                suit: Suit::Clubs,
            },
            Card {
                rank: Rank::Eight,
                suit: Suit::Spades,
            },
            Card {
                rank: Rank::Ace,
                suit: Suit::Diamonds,
            },
        ];
        let abstraction = get_connected_card_abstraction(&hole_cards, &board_cards).unwrap();
        match abstraction {
            ConnectedCardsAbstraction::Pair(pair) => {
                assert_eq!(pair.pair_order_score, 1); // Second pair
            }
            _ => panic!("Expected Pair abstraction"),
        }
    }

    #[test]
    fn test_two_pair_abstraction() {
        let hole_cards = [
            Card {
                rank: Rank::Ten,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Eight,
                suit: Suit::Diamonds,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Ten,
                suit: Suit::Clubs,
            },
            Card {
                rank: Rank::Eight,
                suit: Suit::Spades,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::Ace,
                suit: Suit::Diamonds,
            },
        ];
    }

    #[test]
    fn test_toak_abstraction() {
        let hole_cards = [
            Card {
                rank: Rank::Eight,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Eight,
                suit: Suit::Diamonds,
            },
        ];
        let board_cards = [
            Card {
                rank: Rank::Ten,
                suit: Suit::Clubs,
            },
            Card {
                rank: Rank::Eight,
                suit: Suit::Spades,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::Ace,
                suit: Suit::Diamonds,
            },
        ];

        let abstraction = get_connected_card_abstraction(&hole_cards, &board_cards).unwrap();
        match abstraction {
            ConnectedCardsAbstraction::ThreeOfAKind(toak_abstraction) => {
                assert_eq!(toak_abstraction.toak_order_score, 2); // > Third toak
            }
            _ => panic!("Expected Two pair abstraction"),
        }
    }
}
