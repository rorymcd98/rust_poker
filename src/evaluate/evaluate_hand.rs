use itertools::Itertools;

use crate::models::card::NineCardDeal;
use crate::models::Player;
use crate::Card;
use super::generate_tables::generate_flush_table::generate_flushes_table;
use super::generate_tables::generate_remaining_table::generate_remaining_table;
use super::generate_tables::generate_unique_five_table::generate_unique_five_table;

pub type CardId = u32;
pub type CardIdMask = u32; // 24-bit mask for N cards
pub const PRIME_MASK: u32 = 0b11111111;
pub const SUIT_MASK: u32 = 0b111100000000;

// 7462 = all possible rank combinations (4 * (13 choose 5))
pub const DISTINCT_CARD_COMBOS: usize = 7462;

// 7937 = xxxAKQJTxxxxxxxx (+1) the bit pattern of a royal flush 
// TODO - find a better name
pub const BIT_REP_LIMIT: usize = 7937; // We often add 1 to this so that we can use the 0 index as a sentinel / null value

// 41*41*41*41*37 (product of prime numbers) - this is allocating around 1.6GB; TODO - perfect hashing?
pub const REMAINING_LOOKUP_PRODUCT: usize = 104553157;

/// 32-bit Card representation mostly copied from Cactus Kev
/// xxxA KQJT 9876 5432 SHCD pppp pppp
/// pppp = prime number of rank (deuce=2,trey=3,four=5,five=7,...,ace=41)
pub fn card_to_id(card: &Card) -> CardId {
    let prime = card.rank.to_prime() as u32;

    (1 << (card.rank.to_int() + 12)) | (1 << (card.suit.to_int() + 8)) | prime
}

pub fn id_mask_to_string(id: CardIdMask) -> String {
    let suit = match id & SUIT_MASK {
        0 => "S",
        1 => "H",
        2 => "C",
        3 => "D",
        _ => panic!("Invalid suit"),
    };

    let rank_bits = (id >> 12) as u16;
    let mut mask_string = String::new();
    for i in 0..13 {
        mask_string.push_str(&rank_bits_to_string(rank_bits & 1 << i));
    }
    mask_string + suit
}

fn rank_bits_to_string(rank_bits: u16) -> String {
    let rank = match rank_bits {
        0 => "",
        1 => "2",
        2 => "3",
        4 => "4",
        8 => "5",
        16 => "6",
        32 => "7",
        64 => "8",
        128 => "9",
        256 => "T",
        512 => "J",
        1024 => "Q",
        2048 => "K",
        4096 => "A",
        _ => panic!("Invalid rank {}", rank_bits),
    };
    rank.to_string()
}

pub fn prime_product_to_rank_string(mut product: usize) -> String {
    let mut rank_string = String::new();
    let primes = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41];
    let ranks = ["2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K", "A"];

    while product > 1 {
        let mut found = false;
        for (i, &prime) in primes.iter().enumerate() {
            if product % prime == 0 {
                rank_string.push_str(&format!("{} ", ranks[i]));
                product /= prime;
                found = true;
                break;
            }
        }
        if !found {
            panic!("Invalid prime product");
        }
    }
    rank_string
}

pub fn is_flush(cards: &Vec<CardId>) -> bool {
    (cards[0] & cards[1] & cards[2] & cards[3] & cards[4]) & SUIT_MASK != 0
}

/// Get a representation of the ranks, the unique lookup table will tell us if this is unique
pub fn unique_rank_mask(cards: &Vec<CardId>) -> usize {    
    ((cards[0] | cards[1] | cards[2] | cards[3] | cards[4]) >> 12) as usize
}

pub fn unique_rank_mask_vec(hand: &Vec<CardId>) -> usize {
    let mask = ((&hand[0] | &hand[1] | &hand[2] | &hand[3] | &hand[4]) >> 12) as usize;
    mask
}

pub fn hand_to_unique_prime_product(hand: &[Card]) -> usize {
    hand.iter().map(|card| card.rank.to_prime() as usize).product()
}

trait HandLookup {
    fn flush_evaluation(&self, id: usize) -> u16;
    fn unique_ranks_evaluation(&self, id: usize) -> u16;
    fn remaining_evaluation(&self, id: usize) -> u16;
}

pub struct HandLookupArrays {
    flushes_lookup: [u16; BIT_REP_LIMIT + 1],
    unique_ranks_lookup: [u16; BIT_REP_LIMIT + 1],
    remaining_lookup: Vec<u16>,
}

impl HandLookupArrays {
    pub fn new() -> Self {
        HandLookupArrays {
            flushes_lookup: generate_flushes_table(),
            unique_ranks_lookup: generate_unique_five_table(),
            remaining_lookup: generate_remaining_table(),
        }
    }
}

impl HandLookup for HandLookupArrays {
    fn flush_evaluation(&self, id: usize) -> u16 {
        self.flushes_lookup[id]
    }

    fn unique_ranks_evaluation(&self, id: usize) -> u16 {
        self.unique_ranks_lookup[id]
    }

    fn remaining_evaluation(&self, id: usize) -> u16 {
        self.remaining_lookup[id]
    }
}

/// Evaluate a group of 5 cards to between 1 and 7937
/// 7937 is a Royal Flush
/// 1 is a High Card 7
pub trait HandEvaluator {
    fn evaluate(&self, cards: &Vec<Card>) -> u16;
    fn evaluate_deal(&self, deal: NineCardDeal) -> Option<Player>;
}

pub struct EvaluateHand {
    hand_lookup: HandLookupArrays,
}

impl EvaluateHand {
    pub fn new() -> Self {
        EvaluateHand {
            hand_lookup: HandLookupArrays::new(),
        }
    }
}

impl  HandEvaluator for EvaluateHand {
    fn evaluate(&self, cards: &Vec<Card>) -> u16 {
        let card_ids = cards.iter().map(|c| card_to_id(c)).collect::<Vec<CardId>>();

        let flush = is_flush(&card_ids);
        let unique_rank_representation = unique_rank_mask(&card_ids);
        if flush {
            return self.hand_lookup.flush_evaluation(unique_rank_representation);
        }
        let unique_lookup  =self.hand_lookup.unique_ranks_evaluation(unique_rank_representation);
        if unique_lookup != 0 {
            return unique_lookup;
        } 
        
        let prime_product = (card_ids[0] & PRIME_MASK) * (card_ids[1] & PRIME_MASK) * (card_ids[2] & PRIME_MASK) * (card_ids[3] & PRIME_MASK) * (card_ids[4] & PRIME_MASK);
        return self.hand_lookup.remaining_evaluation(prime_product as usize);
    }

    fn evaluate_deal(&self, deal: NineCardDeal) -> Option<Player> {
        let best_score_traverser = [deal[0], deal[1], deal[4], deal[5], deal[6], deal[7]].iter().combinations(5).map(|combo| combo.into_iter().cloned().collect::<Vec<Card>>()).max_by(|a, b| self.evaluate(a).cmp(&self.evaluate(b)));
        let best_score_opponent = [deal[2], deal[3], deal[4], deal[5], deal[6], deal[7]].iter().combinations(5).map(|combo| combo.into_iter().cloned().collect::<Vec<Card>>()).max_by(|a, b| self.evaluate(a).cmp(&self.evaluate(b)));
        match best_score_traverser.cmp(&best_score_opponent) {
            std::cmp::Ordering::Greater => Some(Player::Traverser),
            std::cmp::Ordering::Less => Some(Player::Opponent),
            std::cmp::Ordering::Equal => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use lazy_static::lazy_static;

    use super::*;
    use crate::evaluate::evaluate_hand::{id_mask_to_string, unique_rank_mask};

    lazy_static! {
        static ref EVALUATOR: EvaluateHand = EvaluateHand::new();
    }

    // Generate 1 million random 5 card hands to assess the performance
    #[test]
    fn test_performance() {
        let start = std::time::Instant::now();
        let hand = Card::new_random_cards(5);
        for _ in 0..1_000_000 {
            let _ = EVALUATOR.evaluate(&hand);
        }
        let duration = start.elapsed();
        assert!(duration.as_secs() < 1);
    }

    #[test]
    fn test_performance_9_card(){
        let hands: Vec<NineCardDeal> = (0..100).map(|_| Card::new_random_9_card_game()).collect();
        let start = std::time::Instant::now();
        for i in 0..1_000_000 {
            let _ = EVALUATOR.evaluate_deal(hands[i % 100 as usize]);
        }
        let duration = start.elapsed();
        assert!(duration.as_secs() < 1);
    }
}