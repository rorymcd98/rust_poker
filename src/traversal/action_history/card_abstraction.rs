use crate::{models::card::Rank, Card};

pub struct PairAbstraction {
    pub pair_order_score: u8, // 0 = top pair, 1 = second pair, 2 = third or more pair
}

pub struct TwoPairAbstraction {
    pub two_pair_order_score: u8, // 0 = top two pair, 1 = top high pair, 2 = mid high pair
}

pub struct ThreeOfAKindAbstraction {
    pub toak_order_score: u8, // 0 = top toak, 1 = second toak, 2 = third or more toak
}

pub struct FullHouseAbstraction {
    pub high_card_is_house: bool,
}

pub struct StraightAbstraction {
    pub high_card: Rank, // Can be further bucketed e.g. A-K, Q-T, 9-5
    pub cards_to_draw: u8, // 0, 1, (& 2 on flop)
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

pub fn get_connected_card_abstraction(hole_cards: &[Card; 2], board_cards: &[Card; 2]) -> Option<ConnectedCardsAbstraction> {
    let mut rank_counts = [0; 13];
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
        return Some(ConnectedCardsAbstraction::FourOfAKind);
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