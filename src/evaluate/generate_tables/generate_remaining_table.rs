use std::collections::HashMap;

use crate::*;
use evaluate::evaluate_hand::hand_to_unique_prime_product;
use itertools::Itertools;
use models::card::{Card, Rank};
use crate::evaluate::evaluate_hand::{REMAINING_LOOKUP_PRODUCT};

use super::remaining_hand_types::{classify_hand_type, evaluate_pair, evaluate_two_pair, evaluate_three_of_a_kind, evaluate_full_house, evaluate_four_of_a_kind, HandType};

fn generate_all_hand_combos(num_cards: usize) -> Vec<Vec<Card>> {
    let combos = (0..52).collect_vec().iter().combinations(num_cards).map(|combo| combo.into_iter().cloned().map(|c| Card::from_int(c)).collect()).collect::<Vec<Vec<Card>>>();
    combos
}

// TODO - Double check the accounting here
// These numbers are just the number of cards in the previous hand type (e.g. 1277 is the number of high-card combos, 2860 is the number of pairs etc.)
const PAIR_OFFSET: usize = 1277 + 1; // +1 to allow for 0 index to equal not found
const TWO_PAIR_OFFSET: usize = 2860 + PAIR_OFFSET;
const THREE_OF_A_KIND_OFFSET: usize = 858 + TWO_PAIR_OFFSET;
const FULL_HOUSE_OFFSET: usize = 1277 + 10 + 858 + THREE_OF_A_KIND_OFFSET;
const FOUR_OF_A_KIND_OFFSET: usize = 156 + FULL_HOUSE_OFFSET;

pub fn generate_remaining_table() -> Vec<u16> {
    let hands = generate_all_hand_combos(5);
    let mut pair_evaluations = HashMap::new();
    let mut two_pair_evaluations = HashMap::new();
    let mut three_of_a_kind_evaluations = HashMap::new();
    let mut full_house_evaluations = HashMap::new();
    let mut four_of_a_kind_evaluations = HashMap::new();

    for hand in hands {
        let prime_product_identifier = hand_to_unique_prime_product(&hand);
        let cards: Vec<Rank> = hand.iter().map(|r| r.rank.clone()).collect();
        let hand_type: HandType = classify_hand_type(&hand);
        match hand_type {
            HandType::Pair(_) => {
                pair_evaluations.insert(prime_product_identifier, evaluate_pair(hand_type, cards));
            },
            HandType::TwoPair(_, _) => {
                two_pair_evaluations.insert(prime_product_identifier, evaluate_two_pair(hand_type, cards));
            },
            HandType::ThreeOfAKind(_) => {
                three_of_a_kind_evaluations.insert(prime_product_identifier, evaluate_three_of_a_kind(hand_type, cards));
            },
            HandType::FullHouse(_, _) => {
                full_house_evaluations.insert(prime_product_identifier, evaluate_full_house(hand_type, cards));
            },
            HandType::FourOfAKind(_) => {
                four_of_a_kind_evaluations.insert(prime_product_identifier, evaluate_four_of_a_kind(hand_type, cards));
            },
            HandType::None => {
                continue;
            }
        }
    };

    let mut remaining_lookup: Vec<u16> = vec![0; REMAINING_LOOKUP_PRODUCT + 1];

    // sort the evaluations and assign their their lookup[prime_product_identifier] to their index + offset
    let mut pair_evaluations = pair_evaluations.into_iter().collect::<Vec<(usize, u32)>>();
    pair_evaluations.sort_by(|a, b| a.1.cmp(&b.1));
    for (idx, (prime_product_identifier, _)) in pair_evaluations.iter().enumerate() {
        remaining_lookup[*prime_product_identifier] = (idx + PAIR_OFFSET) as u16;
    }

    let mut two_pair_evaluations = two_pair_evaluations.into_iter().collect::<Vec<(usize, u32)>>();
    two_pair_evaluations.sort_by(|a, b| a.1.cmp(&b.1));
    for (idx, (prime_product_identifier, _)) in two_pair_evaluations.iter().enumerate() {
        remaining_lookup[*prime_product_identifier] = (idx + TWO_PAIR_OFFSET) as u16;
    }

    let mut three_of_a_kind_evaluations = three_of_a_kind_evaluations.into_iter().collect::<Vec<(usize, u32)>>();
    three_of_a_kind_evaluations.sort_by(|a, b| a.1.cmp(&b.1));
    for (idx, (prime_product_identifier, _)) in three_of_a_kind_evaluations.iter().enumerate() {
        remaining_lookup[*prime_product_identifier] = (idx + THREE_OF_A_KIND_OFFSET) as u16;
    }

    let mut full_house_evaluations = full_house_evaluations.into_iter().collect::<Vec<(usize, u32)>>();
    full_house_evaluations.sort_by(|a, b| a.1.cmp(&b.1));
    for (idx, (prime_product_identifier, _)) in full_house_evaluations.iter().enumerate() {
        remaining_lookup[*prime_product_identifier] = (idx + FULL_HOUSE_OFFSET) as u16;
    }

    let mut four_of_a_kind_evaluations = four_of_a_kind_evaluations.into_iter().collect::<Vec<(usize, u32)>>();
    four_of_a_kind_evaluations.sort_by(|a, b| a.1.cmp(&b.1));
    for (idx, (prime_product_identifier, _)) in four_of_a_kind_evaluations.iter().enumerate() {
        remaining_lookup[*prime_product_identifier] = (idx + FOUR_OF_A_KIND_OFFSET) as u16;
    }

    remaining_lookup
}

#[cfg(test)]
mod tests {
    use super::*;
    use evaluate::generate_tables::generate_flush_table::generate_all_unique_rank_combos;
    use models::card::{Card, Suit};
    use lazy_static::lazy_static;
    use crate::evaluate::evaluate_hand::{prime_product_to_rank_string, DISTINCT_CARD_COMBOS};

    #[cfg(debug_assertions)]
    const EVALS: usize = 1_000;

    #[cfg(not(debug_assertions))]
    const EVALS: usize = 100_000;

    lazy_static! {
        static ref REMAINING_TABLE: Vec<u16> = generate_remaining_table();
    }

    fn evaluate_remaining(hand: &[Card]) -> u16 {
        let prime_product_identifier = hand_to_unique_prime_product(hand);
        let evaluation = REMAINING_TABLE[prime_product_identifier];
        evaluation
    }

    #[test]
    fn test_mutual_exclusivity() {
        let mut count = 0;
        let mut seen_rankings = vec![0; DISTINCT_CARD_COMBOS + 1];
        for (prime_product, ranking) in REMAINING_TABLE.iter().enumerate() {
            if *ranking == 0 {
                continue;
            }
            count += 1;
            if seen_rankings[*ranking as usize] != 0 {
                panic!("Remaining table has duplicate entries {}, conflicts with rank {}", prime_product_to_rank_string(prime_product), ranking);
            }
            seen_rankings[*ranking as usize] += 1;
        }

        assert_eq!(count, 4888); // 1277 high cards + 2860 pairs + 858 two-pair + 156 ToaK + 156 Full House + 26 Four of a kind
    }

    #[test]
    fn test_evaluate_hand1() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Queen),
            Card::new(Suit::random(), Rank::Queen),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Eight),
            Card::new(Suit::random(), Rank::Nine),
        ];
        let prime_product_identifier = hand_to_unique_prime_product(&hand);
        let evaluation = REMAINING_TABLE[prime_product_identifier];
        print!("{:?}", evaluation);
        assert_eq!(evaluation, 3530);
    }

    // Ctrl + Shift + F: "Expected ranges for hand evals"

    #[test]
    fn best_pair() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::King),
            Card::new(Suit::random(), Rank::Queen),
            Card::new(Suit::random(), Rank::Jack),
        ];
        let evaluation = evaluate_remaining(&hand);
        assert_eq!(evaluation, 4137);
    }

    #[test]
    fn worst_pair() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
        ];
        let evaluation = evaluate_remaining(&hand);
        assert_eq!(evaluation, 1278);
    }

    #[test]
    fn best_two_pair() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::King),
            Card::new(Suit::random(), Rank::King),
            Card::new(Suit::random(), Rank::Queen),
        ];
        let evaluation = evaluate_remaining(&hand);
        assert_eq!(evaluation, 4995);
    }

    #[test]
    fn worst_two_pair() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
        ];
        let evaluation = evaluate_remaining(&hand);
        assert_eq!(evaluation, 4138);
    }

    #[test]
    fn best_three_of_a_kind() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::King),
            Card::new(Suit::random(), Rank::Queen),
        ];
        let evaluation = evaluate_remaining(&hand);
        assert_eq!(evaluation, 5853);
    }

    #[test]
    fn worst_three_of_a_kind() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
        ];
        let evaluation = evaluate_remaining(&hand);
        assert_eq!(evaluation, 4996);
    }

    #[test]
    fn best_full_house() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::King),
            Card::new(Suit::random(), Rank::King),
        ];
        let evaluation = evaluate_remaining(&hand);
        assert_eq!(evaluation, 7296);
    }

    #[test]
    fn worst_full_house() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Three),
        ];
        let evaluation = evaluate_remaining(&hand);
        assert_eq!(evaluation, 7141);
    }

    #[test]
    fn best_four_of_a_kind() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::King),
        ];
        let evaluation = evaluate_remaining(&hand);
        assert_eq!(evaluation, 7452);
    }

    #[test]
    fn worst_four_of_a_kind() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
        ];
        let evaluation = evaluate_remaining(&hand);
        assert_eq!(evaluation, 7297);
    }


    #[test]
    fn all_straights_and_highcards_are_zero(){
        let straights = generate_all_unique_rank_combos(5);
        for straight in straights {
            let evaluation = evaluate_remaining(&straight);
            assert_eq!(evaluation, 0);
        }
    }

    // A test to show the hand evaluation and board evaluation order doesn't mattter
    #[test]
    fn order_invariance_hand(){
        for _ in 0..EVALS {
            let hand = Card::new_random_cards(5);
            let first_eval = evaluate_remaining(&hand);
            for perm in hand.into_iter().permutations(5) {
                assert_eq!(first_eval, evaluate_remaining(&perm));
            }
        }
    }

    #[test]
    fn test_correct_bucketing(){
        for _ in 0..EVALS {
            let hand = Card::new_random_cards(5);
            let eval = evaluate_remaining(&hand);
            let hand_type = classify_hand_type(&hand);
            match hand_type {
                HandType::None => assert!(eval == 0),
                HandType::Pair(_) => assert!(eval < 4138 && eval >= 1278),
                HandType::TwoPair(_, _) => assert!(eval < 4996 && eval >= 4138),
                HandType::ThreeOfAKind(_) => assert!(eval < 5854 && eval >= 4996),
                HandType::FullHouse(_, _) => assert!(eval < 7297 && eval >= 7141),
                HandType::FourOfAKind(_) => assert!(eval < 7453 && eval >= 7297),
            }
        }
    }
}