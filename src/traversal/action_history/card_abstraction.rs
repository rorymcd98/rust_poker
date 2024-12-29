use crate::models::card::{Card, Rank};

pub struct StraightAbstraction {
    //TODO - there will be noise here potentially (?) Q6 and J6  with the board being T987 - do we resolve the right strategy here? 
    pub bucketed_high_card: Rank, // Can be further bucketed e.g. A-K, Q-T, 9-5 
    pub cards_in_straight: u8, // 0, 1, (& 2 on flop)
    pub requires_gutshot: bool,
}

// Bucket ints to A-K, Q-T, 9-5 
fn round_rank(rank_int: usize) -> Rank {
    match rank_int {
        4..8 => Rank::Nine,
        8..11 => Rank::Queen,
        11..13 => Rank::Ace,
        _ => panic!("Invalid rank int")
    }
}

// Find the highest straight that is either length 4 with a gutshot, or length 3-5 without a gutshot
pub fn get_straight_abstraction(hole_cards: &[Card; 2], board_cards: &[Card]) -> Option<StraightAbstraction> {
    let mut rank_counts = [false; 13];
    for card in board_cards {
        let idx = card.rank.to_int() as usize;
        rank_counts[idx] = true;
    }
    let mut candidate_hole_cards_indeces = vec![];
    for card in hole_cards {
        let rank_index = card.rank.to_int() as usize;
        if rank_counts[rank_index] {
            candidate_hole_cards_indeces.push(rank_index);
            rank_counts[rank_index] = true;
        }
    }

    if candidate_hole_cards_indeces.is_empty() {
        return None;
    }

    let mut highest_straight_without_gutshot = 0;
    let mut connected_high_card = 0;

    let mut highest_straight_with_gutshot = 0;
    let mut gutshot_high_card = 0;

    // roll a len 5 window over the ranks checking if it's a straight, and updating either len 4 gutshots, or len 3+ otherwise
    let mut in_window = 0;
    let mut consecutive = 0;
    
    for i in 0..4 {
        if i < 4 {
            if rank_counts[i] {
                in_window += 1;
                consecutive += 1;

                if candidate_hole_cards_indeces.iter().any(|&x| x <= i && x >= i.saturating_sub(4)) && consecutive >= 3 {
                    highest_straight_without_gutshot = consecutive;
                    connected_high_card = i;
                }
            } else {
                consecutive = 0;
            }
        } 
    }
    for i in 4..13 {
        if rank_counts[i] {
            in_window += 1;
            consecutive += 1;
            if candidate_hole_cards_indeces.iter().any(|&x| x <= i && x >= i - 4) {
                if in_window >= 3 && consecutive >= 3 {
                    highest_straight_without_gutshot = consecutive;
                    connected_high_card = i;
                } else if in_window == 4 && consecutive < 4 {
                    highest_straight_with_gutshot = 4;
                    gutshot_high_card = i;
                }
            }
        } else {
            consecutive = 0;
        }
    }

    if highest_straight_with_gutshot > highest_straight_without_gutshot {
        Some(StraightAbstraction { bucketed_high_card: round_rank(gutshot_high_card), cards_in_straight: highest_straight_with_gutshot, requires_gutshot: true})
    } else if highest_straight_without_gutshot > 0 {
        return Some(StraightAbstraction { bucketed_high_card: round_rank(connected_high_card), cards_in_straight: highest_straight_without_gutshot, requires_gutshot: false});
    } else {
        return None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_rank() {
        assert_eq!(round_rank(4), Rank::Nine);
        assert_eq!(round_rank(8), Rank::Queen);
        assert_eq!(round_rank(11), Rank::Ace);
    }

    #[test]
    #[should_panic(expected = "Invalid rank int")]
    fn test_round_rank_invalid() {
        round_rank(13);
    }

    #[test]
    fn test_get_straight_abstraction() {
        let hole_cards = [
            Card { rank: Rank::Ten, suit: Suit::Hearts },
            Card { rank: Rank::Jack, suit: Suit::Hearts },
        ];
        let board_cards = [
            Card { rank: Rank::Nine, suit: Suit::Clubs },
            Card { rank: Rank::Eight, suit: Suit::Diamonds },
            Card { rank: Rank::Seven, suit: Suit::Spades },
        ];
        let abstraction = get_straight_abstraction(&hole_cards, &board_cards).unwrap();
        assert_eq!(abstraction.bucketed_high_card, Rank::Queen); // rounded
        assert_eq!(abstraction.cards_in_straight, 5);
        assert_eq!(abstraction.requires_gutshot, false);
    }

    #[test]
    fn test_get_straight_abstraction_with_gutshot() {
        let hole_cards = [
            Card { rank: Rank::Six, suit: Suit::Hearts },
            Card { rank: Rank::Queen, suit: Suit::Hearts },
        ];
        let board_cards = [
            Card { rank: Rank::Ten, suit: Suit::Clubs },
            Card { rank: Rank::Nine, suit: Suit::Diamonds },
            Card { rank: Rank::Eight, suit: Suit::Spades },
            Card { rank: Rank::Seven, suit: Suit::Hearts },
        ];
        let abstraction = get_straight_abstraction(&hole_cards, &board_cards).unwrap();
        assert_eq!(abstraction.bucketed_high_card, Rank::Queen);
        assert_eq!(abstraction.cards_in_straight, 4);
        assert_eq!(abstraction.requires_gutshot, true);
    }

    #[test]
    fn test_get_connected_card_abstraction() {
        let hole_cards = [
            Card { rank: Rank::Ten, suit: Suit::Hearts },
            Card { rank: Rank::Ten, suit: Suit::Diamonds },
        ];
        let board_cards = [
            Card { rank: Rank::Ten, suit: Suit::Clubs },
            Card { rank: Rank::Nine, suit: Suit::Diamonds },
            Card { rank: Rank::Nine, suit: Suit::Spades },
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
    fn test_get_connected_card_abstraction_four_of_a_kind() {
        let hole_cards = [
            Card { rank: Rank::Ten, suit: Suit::Hearts },
            Card { rank: Rank::Ten, suit: Suit::Diamonds },
        ];
        let board_cards = [
            Card { rank: Rank::Ten, suit: Suit::Clubs },
            Card { rank: Rank::Ten, suit: Suit::Spades },
            Card { rank: Rank::Nine, suit: Suit::Diamonds },
        ];
        let abstraction = get_connected_card_abstraction(&hole_cards, &board_cards).unwrap();
        match abstraction {
            ConnectedCardsAbstraction::FourOfAKind => {}
            _ => panic!("Expected FourOfAKind abstraction"),
        }
    }
}

pub struct FlushAbstraction {
    pub high_card: Rank, // Can be further bucketed e.g. A-K, Q-T, 9-5
    pub matches_players_high_card: bool, // Will always be true for suited hands
    pub cards_to_draw: u8, // 0, 1, (& 2 on flop)
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

pub fn get_connected_card_abstraction(hole_cards: &[Card; 2], board_cards: &[Card]) -> Option<ConnectedCardsAbstraction> {
    let mut rank_counts = [0u8; 13];
    for card in hole_cards {
        let idx = card.rank.to_int() as usize;
        rank_counts[idx] += 1;
    }
    for card in board_cards {
        let idx = card.rank.to_int() as usize;
        if rank_counts[idx] > 0 {
            rank_counts[idx] += 1;
        }
    }

    let mut highest_pair_rank = 0;
    let mut highest_toak_rank = 0;
    let mut seen_so_far_pair = 0;
    let mut seen_so_far_toak = 0;
    let mut seen_so_far = 0;

    let mut counts = [0; 3];
    for i in 13..=0 {
        let rank_count = rank_counts[i];
        match rank_count {
            2 => {
                highest_pair_rank = highest_pair_rank.max(i+1);
                seen_so_far_pair = seen_so_far_pair.min(seen_so_far);
            },
            3 => {
                highest_toak_rank = highest_toak_rank.max(i+1);
                seen_so_far_toak = seen_so_far_toak.min(seen_so_far);
            },
            _ => {}
        }
        if rank_count > 1 {
            counts[rank_count as usize - 2] += 1;
        }
        seen_so_far += rank_count.max(1);
    }

    if counts[2] > 0 {
        Some(ConnectedCardsAbstraction::FourOfAKind)
    } else if counts[1] > 0 && counts[0] > 0 {
        return Some(ConnectedCardsAbstraction::FullHouse(FullHouseAbstraction { high_card_is_house: highest_pair_rank > highest_toak_rank }));
    } else if counts[1] > 0 {
        return Some(ConnectedCardsAbstraction::ThreeOfAKind(ThreeOfAKindAbstraction { toak_order_score: seen_so_far_toak.max(2) })); // Here we classify some Full houses as ToaKs, but it shouldn't matter since the board pairing will contain info about pairs and other ToakS
    } else if counts[0] > 1 {
        return Some(ConnectedCardsAbstraction::TwoPair(TwoPairAbstraction { two_pair_order_score: seen_so_far_pair.max(2) }));
    } else if counts[0] > 0 {
        return Some(ConnectedCardsAbstraction::Pair(PairAbstraction { pair_order_score: seen_so_far_pair.max(2) }));
    } else {
        return None;   
    }
}

pub struct HoleCardsAbstraction {
    pub lower_card: Rank,
    pub higher_card: Rank,
    pub suited: bool,
}