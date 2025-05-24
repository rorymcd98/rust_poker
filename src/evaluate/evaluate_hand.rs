use std::u16;

use itertools::Itertools;

use super::generate_tables::generate_flush_table::generate_flushes_table;
use super::generate_tables::generate_remaining_table::generate_remaining_table;
use super::generate_tables::generate_unique_five_table::generate_unique_five_table;
use crate::models::Card;
use crate::models::Player;

pub type CardId = u32;
pub const PRIME_MASK: u32 = 0b11111111;
pub const SUIT_MASK: u32 = 0b111100000000;

// 7462 = all possible rank combinations (4 * (13 choose 5))
#[cfg(test)]
pub const DISTINCT_CARD_COMBOS: usize = 7462;

// 7937 = xxxAKQJTxxxxxxxx (+1) the bit pattern of a royal flush
pub const BIT_REP_LIMIT: usize = 7937; // We often add 1 to this so that we can use the 0 index as a sentinel / null value

// 41*41*41*41*37 (product of prime numbers) - this is allocating around 1.6GB; TODO - perfect hashing?
pub const REMAINING_LOOKUP_PRODUCT: usize = 104553157;

/// 32-bit Card representation mostly copied from Cactus Kev
/// xxxA KQJT 9876 5432 SHCD pppp pppp
/// pppp = prime number of rank (deuce=2,trey=3,four=5,five=7,...,ace=41)
pub fn card_to_id(card: &Card) -> CardId {
    let prime = card.rank.to_prime() as u32;

    (card.rank.to_bit() << 12) | (card.suit.to_bit() << 8) | prime
}

#[cfg(test)]
pub fn id_mask_to_string(id: CardId) -> String {
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

#[cfg(test)]
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

#[cfg(test)]
pub fn prime_product_to_rank_string(mut product: usize) -> String {
    let mut rank_string = String::new();
    let primes = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41];
    let ranks = [
        "2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K", "A",
    ];

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

pub fn is_flush(cards: &[CardId; 5]) -> bool {
    (cards[0] & cards[1] & cards[2] & cards[3] & cards[4]) & SUIT_MASK != 0
}

/// Get a representation of the ranks, the unique lookup table will tell us if this is unique
pub fn unique_rank_mask(cards: &[CardId; 5]) -> usize {
    ((cards[0] | cards[1] | cards[2] | cards[3] | cards[4]) >> 12) as usize
}

pub fn hand_to_unique_prime_product(hand: &[Card]) -> usize {
    hand.iter()
        .map(|card| card.rank.to_prime() as usize)
        .product()
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

/// Evaluate a group of 5 cards to between 1 and 7463
/// 7463 is a Royal Flush
/// 1 is a High Card 7
pub trait HandEvaluator {
    /// Evaluate a 5 card hand into its absolute rank amongs all 5 card hands 
    fn evaluate_five(&self, cards: [Card; 5]) -> u16;
    /// Evaluate a 7 card deal into its maximum 5 card deal 
    /// The short_circuit parameter allows us to return early if we encounter a greater value
    fn evaluate_seven(&self, hole_cards: &[Card; 2], board_cards: &[Card; 5], short_circuit: u16) -> u16;
    /// Evaluate which player has the best 7 card deal [2 Traverser cards] [2 Opponent cards] [7 Board cards]
    fn evaluate_nine(&self, deal: &[Card; 9]) -> Option<Player>;
}

pub struct HandEvaluatorLookup {
    hand_lookup: HandLookupArrays,
}

impl HandEvaluatorLookup {
    pub fn new() -> Self {
        println!("Generating hand lookup tables...");
        HandEvaluatorLookup {
            hand_lookup: HandLookupArrays::new(),
        }
    }
}

pub fn hand_to_id(hand: &[Card]) -> [CardId; 5] {
    [
        card_to_id(&hand[0]),
        card_to_id(&hand[1]),
        card_to_id(&hand[2]),
        card_to_id(&hand[3]),
        card_to_id(&hand[4]),
    ]
}

impl HandEvaluator for HandEvaluatorLookup {
    fn evaluate_five(&self, cards: [Card; 5]) -> u16 {
        let card_ids = hand_to_id(&cards);
        let flush = is_flush(&card_ids);
        let unique_rank_representation = unique_rank_mask(&card_ids);
        if flush {
            return self
                .hand_lookup
                .flush_evaluation(unique_rank_representation);
        }
        let unique_lookup = self
            .hand_lookup
            .unique_ranks_evaluation(unique_rank_representation);
        if unique_lookup != 0 {
            return unique_lookup;
        }

        let prime_product = (card_ids[0] & PRIME_MASK)
            * (card_ids[1] & PRIME_MASK)
            * (card_ids[2] & PRIME_MASK)
            * (card_ids[3] & PRIME_MASK)
            * (card_ids[4] & PRIME_MASK);
        self.hand_lookup
            .remaining_evaluation(prime_product as usize)
    }

    fn evaluate_nine(&self, deal: &[Card; 9]) -> Option<Player> {
        let board: &[Card; 5] = deal[4..9].try_into().expect("Board is not 5 cards");
        let traverser_cards: &[Card; 2] = deal[0..2].try_into().expect("Traverser cards are not 2 cards");
        let opponenet_cards: &[Card; 2] = deal[2..4].try_into().expect("Opponent cards are not 2 cards");

        let best_score_traverser = self.evaluate_seven(traverser_cards, board, u16::MAX);
        let best_score_opponent = self.evaluate_seven(opponenet_cards, board, best_score_traverser);

        match best_score_traverser.cmp(&best_score_opponent) {
            std::cmp::Ordering::Greater => Some(Player::Traverser),
            std::cmp::Ordering::Less => Some(Player::Opponent),
            std::cmp::Ordering::Equal => None,
        }
    }

    fn evaluate_seven(&self, hole_cards: &[Card; 2], board_cards: &[Card; 5], short_circuit: u16) -> u16 {
        let mut cards = Vec::with_capacity(7);
        cards.extend_from_slice(hole_cards);
        cards.extend_from_slice(board_cards);
    
        let mut max_score = 0;

        if short_circuit == u16::MAX{
            for combo in cards.iter().combinations(5) {
                let hand = [
                    *combo[0],
                    *combo[1],
                    *combo[2],
                    *combo[3],
                    *combo[4],
                ];
                let score = self.evaluate_five(hand);
                max_score = max_score.max(score);
            }
        } else {
            for combo in cards.iter().combinations(5) {
                let hand = [
                    *combo[0],
                    *combo[1],
                    *combo[2],
                    *combo[3],
                    *combo[4],
                ];
                let score = self.evaluate_five(hand);
                if short_circuit != u16::MAX && score > short_circuit {
                    return score;
                }
                max_score = max_score.max(score);
            }
        }
        max_score
    }
}

// Expected ranges for hand evals
// High Card:              0 - 1277
// One pair:            1277 - 4137
// Two pair:            4137 - 4995
// Three-of-a-kind:     4995 - 5853
// Straight:            5853 - 5863
// Flush:               5863 - 7140
// Full house:          7140 - 7296
// Four of a kind:      7296 - 7452
// Straight flush:      7452 - 7462
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::card::{new_random_nine_card_game, NineCardDeal, Rank, Suit};
    use itertools::Itertools;
    use lazy_static::lazy_static;
    use rand::seq::SliceRandom;

    lazy_static! {
        static ref EVALUATOR: HandEvaluatorLookup = HandEvaluatorLookup::new();
    }

    #[cfg(debug_assertions)]
    const EVALS: usize = 1_000;

    #[cfg(not(debug_assertions))]
    const EVALS: usize = 100_000;

    #[test]
    fn test_performance() {
        _ = &*EVALUATOR;
        let start = std::time::Instant::now();
        let hand = Card::new_random_cards(5);
        for _ in 0..EVALS {
            let _ = EVALUATOR.evaluate_five([hand[0], hand[1], hand[2], hand[3], hand[4]]);
        }
        let duration = start.elapsed();
        assert!(duration.as_secs() < 1);
    }

    #[test]
    fn test_performance_9_card() {
        _ = &*EVALUATOR;
        let hands: Vec<NineCardDeal> = (0..100).map(|_| new_random_nine_card_game()).collect();
        let start = std::time::Instant::now();
        for i in 0..EVALS {
            let _ = EVALUATOR.evaluate_nine(&hands[i % 100]);
        }
        let duration = start.elapsed();
        println!("9 card performance test took {:?}", duration);
        assert!(duration.as_secs() < 10);
    }

    // A test to show the hand evaluation and board evaluation order doesn't mattter
    #[test]
    fn order_invariance_hand() {
        for _ in 0..EVALS {
            let hand = Card::new_random_cards(5);
            let first_eval = EVALUATOR.evaluate_five([hand[0], hand[1], hand[2], hand[3], hand[4]]);
            for perm in hand.iter().permutations(5) {
                assert_eq!(
                    first_eval,
                    EVALUATOR.evaluate_five([*perm[0], *perm[1], *perm[2], *perm[3], *perm[4]])
                );
            }
        }
    }

    #[test]
    fn order_invariance_nine_card_game() {
        for _ in 0..EVALS {
            let game = new_random_nine_card_game();
            let first_eval = EVALUATOR.evaluate_nine(&game);
            let mut rng = rand::thread_rng();
            for perm in game[4..9].iter().permutations(5) {
                let mut game_perm = [
                    game[0], game[1], game[2], game[3], *perm[0], *perm[1], *perm[2], *perm[3],
                    *perm[4],
                ];
                game_perm[0..2].shuffle(&mut rng);
                game_perm[2..4].shuffle(&mut rng);
                assert_eq!(first_eval, EVALUATOR.evaluate_nine(&game_perm));
            }
        }
    }

    #[test]
    fn split_pot_straight() {
        let straight_deal = [
            // Traverser cards
            Card::new(Suit::Spades, Rank::Jack),
            Card::new(Suit::Clubs, Rank::Jack),

            // Opponent cards
            Card::new(Suit::Spades, Rank::King),
            Card::new(Suit::Clubs, Rank::King),

            Card::new(Suit::Hearts, Rank::Two),
            Card::new(Suit::Diamonds, Rank::Three),
            Card::new(Suit::Hearts, Rank::Four),
            Card::new(Suit::Diamonds, Rank::Five),
            Card::new(Suit::Hearts, Rank::Six),
        ];

        let result = EVALUATOR.evaluate_nine(&straight_deal);
        assert_eq!(result, None);
    }
}
