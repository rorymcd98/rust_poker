use crate::Card;
use crate::Suit;
use crate::Rank;

pub type CardId = u32;

pub const PRIME_MASK: u32 = 0b00001111;
pub const SUIT_MASK: u32 = 0b11110000;
pub const DISTINCT_COUNT: usize = 7937;
pub const REMAINING_LOOKUP_PRODUCT: usize = 104553157;

/// 24-bit Card representation mostly copied from Cactus Kev
/// xxxA KQJT 9876 5432 SHCD pppp
/// pppp = prime number of rank (deuce=2,trey=3,four=5,five=7,...,ace=41)
pub fn card_to_id(card: &Card) -> CardId {
    let prime = card.rank.to_prime() as u32;

    (1 << (card.rank.to_int() + 8)) | (1 << (card.suit.to_int() + 4)) | prime
}

pub fn id_to_card_string(id: CardId) -> String {
    let rank = match (id & 0b111111111111100000000) >> 8 {
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
        _ => panic!("Invalid rank"),
    };

    let suit = match (id >> 4) & 0b1111 {
        0 => "s",
        1 => "h",
        2 => "c",
        3 => "d",
        _ => panic!("Invalid suit"),
    };

    format!("{}{}", rank, suit)
}

pub fn prime_product_to_rank_string(mut product: usize) -> String {
    let mut rank_string = String::new();
    let primes = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41];
    let ranks = ["2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K", "A"];

    while product > 1 {
        let mut found = false;
        for (i, &prime) in primes.iter().enumerate() {
            if product % prime == 0 {
                rank_string.push_str(ranks[i]);
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

pub fn is_flush(cards: &[CardId]) -> bool {
    let flush = (cards[0] & cards[1] & cards[2] & cards[3] & cards[4]) & SUIT_MASK;
    flush != 0
}

/// Get a representation of the ranks, the unique lookup table will tell us if this is unique
pub fn unique_rank_mask(cards: &[CardId]) -> usize {    
    ((cards[0] | cards[1] | cards[2] | cards[3] | cards[4]) >> 8) as usize
}

pub fn unique_rank_mask_vec(hand: &Vec<CardId>) -> usize {
    let mask = ((&hand[0] | &hand[1] | &hand[2] | &hand[3] | &hand[4]) >> 8) as usize;
    mask
}

pub fn hand_to_unique_prime_product(hand: &Vec<Card>) -> usize {
    let mut product = 1;
    for card in hand {
        product *= card.rank.to_prime() as usize;
    }
    product
}

trait HandLookup {
    fn get_flush_rank(&self, id: usize) -> u16;
    fn get_unique_ranks(&self, id: usize) -> u16;
    fn get_remaining(&self, id: usize) -> u16;
}

pub struct HandLookupArrays {
    flushes_lookup: [u16; DISTINCT_COUNT + 1], // 7937 = xxxAKQJTxxxxxxxx (+1) the bit pattern of a royal flush 
    unique_ranks_lookup: [u16; DISTINCT_COUNT + 1],
    remaining_lookup: [u16; REMAINING_LOOKUP_PRODUCT], // 41*41*41*41*37 (product of prime numbers) - this is around 1.6GB; TODO - perfect hashing
}

impl HandLookupArrays {
    pub fn new(flushes_lookup_path: String, unique_ranks_lookup_path: String, remaining_lookup_path: String) -> HandLookupArrays {
        HandLookupArrays {
            flushes_lookup: [0; DISTINCT_COUNT + 1],
            unique_ranks_lookup: [0; DISTINCT_COUNT + 1],
            remaining_lookup: [0; REMAINING_LOOKUP_PRODUCT],
        }
    }
}

impl HandLookup for HandLookupArrays {
    fn get_flush_rank(&self, id: usize) -> u16 {
        self.flushes_lookup[id]
    }

    fn get_unique_ranks(&self, id: usize) -> u16 {
        self.unique_ranks_lookup[id]
    }

    fn get_remaining(&self, id: usize) -> u16 {
        self.remaining_lookup[id]
    }
}

/// Evaluate a group of 5 cards to between 1 and 7462
/// 7462 is a Royal Flush
/// 1 is a High Card 7
pub trait HandEvaluator {
    fn evaluate(&self, cards: &[Card]) -> u16;
}

pub struct EvaluateHand {
    hand_lookup: dyn HandLookup,
}

impl  HandEvaluator for EvaluateHand {
    fn evaluate(&self, cards: &[Card]) -> u16 {
        let card_ids = cards.iter().map(|c| card_to_id(c)).collect::<Vec<CardId>>();

        let flush = is_flush(&card_ids);
        let unique_rank_representation = unique_rank_mask(&card_ids);
        if flush {
            return self.hand_lookup.get_flush_rank(unique_rank_representation);
        }
        let unique_lookup  =self.hand_lookup.get_unique_ranks(unique_rank_representation);
        if unique_lookup != 0 {
            return self.hand_lookup.get_unique_ranks(unique_rank_representation);
        } 
        
        let product = (card_ids[0] & PRIME_MASK) * (card_ids[1] & PRIME_MASK) * (card_ids[2] & PRIME_MASK) * (card_ids[3] & PRIME_MASK) * (card_ids[4] & PRIME_MASK);
        return self.hand_lookup.get_remaining(product as usize);
    }
}

