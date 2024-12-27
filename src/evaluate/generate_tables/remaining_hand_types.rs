use crate::{models::card::Rank, Card};
use itertools::Itertools;

/// Hand evaluation for cards which aren't flushes, straights, or high cards
#[derive(Debug, PartialEq, Eq)]
pub enum HandType {
    Pair(Rank),
    TwoPair(Rank, Rank),
    ThreeOfAKind(Rank),
    FullHouse(Rank, Rank),
    FourOfAKind(Rank),
    None
}

pub fn classify_hand_type (hand: &Vec<Card>) -> HandType {
    let mut rank_counts = [0; 13];
    for card in hand {
        let idx = card.rank.to_int() as usize;
        rank_counts[idx] += 1;
    }

    let mut pair1 = None;
    let mut pair2 = None;
    let mut three_of_a_kind = None;
    let mut four_of_a_kind = None;
    for (idx, count) in rank_counts.iter().enumerate() {
        match count {
            2 => {
                if pair1.is_none() {
                    pair1 = Some(Rank::from_int(idx as u8));
                } else {
                    pair2 = Some(Rank::from_int(idx as u8));
                }
            },
            3 => {
                three_of_a_kind = Some(Rank::from_int(idx as u8));
            },
            4 => {
                four_of_a_kind = Some(Rank::from_int(idx as u8));
            },
            _ => {},
        };
    }

    if let Some(four_of_a_kind) = four_of_a_kind {
        return HandType::FourOfAKind(four_of_a_kind);
    }
    if let Some(three_of_a_kind) = three_of_a_kind {
        if let Some(pair1) = pair1 {
            return HandType::FullHouse(three_of_a_kind, pair1);
        }
        return HandType::ThreeOfAKind(three_of_a_kind);
    }
    if let Some(pair1) = pair1 {
        if let Some(pair2) = pair2 {
            // swap so the bigger pair is first
            if pair1.to_int() < pair2.to_int() {
                return HandType::TwoPair(pair2, pair1);
            }
            return HandType::TwoPair(pair1, pair2);
        }
        return HandType::Pair(pair1);
    }
    HandType::None
}

#[cfg(test)]
mod classify_hands_tests {
    use super::*;
    use crate::*;

    #[test]
    fn test_pair() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
        ];
        let classification = classify_hand_type(&hand);
        assert!(matches!(classification, HandType::Pair(Rank::Two)));
    }

    #[test]
    fn test_three_of_a_kind() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
        ];
        let classification = classify_hand_type(&hand);
        assert!(matches!(classification, HandType::ThreeOfAKind(Rank::Four)));
    }

    #[test]
    fn test_two_pair() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Five),
        ];
        let classification = classify_hand_type(&hand);
        assert!(matches!(classification, HandType::TwoPair(Rank::Three, Rank::Two) | HandType::TwoPair(Rank::Two, Rank::Three)));
    }

    #[test]
    fn test_full_house() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Three),
        ];
        let classification = classify_hand_type(&hand);
        assert!(matches!(classification, HandType::FullHouse(Rank::Three, Rank::Two)));
    }

    #[test]
    fn test_four_of_a_kind() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Five),
        ];
        let classification = classify_hand_type(&hand);
        assert!(matches!(classification, HandType::FourOfAKind(Rank::Ace)));
    }

    #[test]
    fn test_none_type() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
        ];
        let classification = classify_hand_type(&hand);
        assert_eq!(classification, HandType::None);
    }
}

// Evaluate the remaining cards that don't form part of the pair, trip etc.
fn evaluate_high_cards(hand: &Vec<Rank>, skip: &Vec<Rank>) -> u32 { // maximum product is 41*37*31 = 47,027 which is conveniently 16 bits
    let mut prime_product: u16 = 1;
    for rank in hand {
        if skip.contains(rank) {
            continue;
        }
        prime_product *= rank.to_prime() as u16; // keep this at 16 bits to check for int overflow
    }
    prime_product as u32
}

pub fn evaluate_pair(pair: HandType, cards: Vec<Rank>) -> u32 {
    match pair {
        HandType::Pair(rank) => {
            return (rank.to_prime() as u32) << 16 | evaluate_high_cards(&cards, &vec![rank]);
        },
        _ => panic!("Unexepected hand type"),
    }
}

pub fn evaluate_two_pair(two_pair: HandType, cards: Vec<Rank>) -> u32 {
    match two_pair {
        HandType::TwoPair(rank1, rank2) => {
            return (rank1.to_prime() as u32) << 24 | (rank2.to_prime() as u32) << 16 | evaluate_high_cards(&cards, &vec![rank1, rank2]);
        },
        _ => panic!("Unexepected hand type"),
    }
}

pub fn evaluate_three_of_a_kind(pair: HandType, cards: Vec<Rank>) -> u32 {
    match pair {
        HandType::ThreeOfAKind(rank) => {
            return (rank.to_prime() as u32) << 16 | evaluate_high_cards(&cards, &vec![rank]);
        },
        _ => panic!("Unexepected hand type"),
    }
}

pub fn evaluate_full_house(full_house: HandType, cards: Vec<Rank>) -> u32 {
    match full_house {
        HandType::FullHouse(rank1, rank2) => {
            return (rank1.to_prime() as u32) << 24 | (rank2.to_prime() as u32) << 16 | evaluate_high_cards(&cards, &vec![rank1, rank2]);
        },
        _ => panic!("Unexepected hand type"),
    }
}

pub fn evaluate_four_of_a_kind(pair: HandType, cards: Vec<Rank>) -> u32 {
    match pair {
        HandType::FourOfAKind(rank) => {
            return (rank.to_prime() as u32) << 16 | evaluate_high_cards(&cards, &vec![rank]);
        },
        _ => panic!("Unexepected hand type"),
    }
}