use crate::evaluate::evaluate_hand::hand_to_id;
use crate::evaluate::evaluate_hand::unique_rank_mask;
use crate::evaluate::evaluate_hand::BIT_REP_LIMIT;
use crate::models::card::Rank;
use crate::models::Card;
use crate::models::Suit;
use itertools::Itertools;
use std::collections::HashMap;

pub fn generate_all_unique_rank_combos(num_cards: usize) -> Vec<Vec<Card>> {
    let ranks = (0..13).collect::<Vec<u8>>();
    let mut combos = Vec::new();

    for combo in ranks.into_iter().combinations(num_cards) {
        let mut cards = Vec::new();
        for rank in combo {
            cards.push(Card::new(Suit::random(), Rank::from_int(rank)));
        }
        combos.push(cards);
    }

    combos
}

#[cfg(test)]
pub const NON_STRAIGHT_COUNT: usize = 1277; // The number of hands consisting of 5 unique cards which are not straights
#[cfg(test)]
pub const STRAIGHT_COUNT: usize = 10;

#[cfg(test)]
mod uniques_generation_test {
    use crate::evaluate::evaluate_hand::hand_to_id;

    use super::*;
    #[test]
    fn test_generate_all_unique_rank_combos() {
        let combos = generate_all_unique_rank_combos(5);
        assert_eq!(combos.len(), STRAIGHT_COUNT + NON_STRAIGHT_COUNT);
    }

    #[test]
    fn test_all_hands_unique() {
        let combos = generate_all_unique_rank_combos(5);
        let mut seen = std::collections::HashSet::new();
        for hand in combos {
            let hand_id = hand_to_id(&hand);
            let unique_hand_mask = unique_rank_mask(&hand_id);
            assert!(
                !seen.contains(&unique_hand_mask),
                "Duplicate hand found, {:?}",
                hand
            );
            seen.insert(unique_hand_mask);
        }
    }
}

fn is_straight(hand: &Vec<Card>) -> bool {
    let mut ranks = [0u8; 14]; // 0 is for Ace, 14 is for high
    for card in hand {
        ranks[1 + card.rank.to_int() as usize] += 1;
        if card.rank == Rank::Ace {
            ranks[0] += 1; // Ace can be high and low
        }
    }
    let mut count = 0;
    for rank in ranks.iter() {
        if *rank > 0 {
            count += 1;
        } else {
            count = 0;
        }
        if count == 5 {
            return true;
        }
    }
    false
}

fn evaluate_straight(hand: &Vec<Card>) -> u32 {
    let mut prod = 1;
    for card in hand {
        prod *= card.rank.to_int() as u32;
    }
    // if the prod is 0 then it means we have a 2, check for an Ace
    if prod == 0 && hand.iter().any(|card| card.rank == Rank::Ace) {
        return 0;
    }

    prod + 1
}

#[cfg(test)]
mod straight_test {
    use super::*;

    #[test]
    fn test_straight() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
        ];
        assert!(is_straight(&hand));
    }

    #[test]
    fn test_straight_2() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Jack),
            Card::new(Suit::random(), Rank::Queen),
            Card::new(Suit::random(), Rank::Ten),
            Card::new(Suit::random(), Rank::King),
            Card::new(Suit::random(), Rank::Ace),
        ];
        assert!(is_straight(&hand));
    }

    #[test]
    fn test_straight_3() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Ten),
            Card::new(Suit::random(), Rank::Queen),
            Card::new(Suit::random(), Rank::Jack),
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::King),
        ];
        assert!(is_straight(&hand));
    }

    #[test]
    fn test_not_straight() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Eight),
            Card::new(Suit::random(), Rank::Ten),
            Card::new(Suit::random(), Rank::Jack),
            Card::new(Suit::random(), Rank::Queen),
            Card::new(Suit::random(), Rank::King),
        ];
        assert!(!is_straight(&hand));
    }

    #[test]
    fn test_straight_eval() {
        let straight1 = vec![
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
        ];
        let straight2 = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
        ];
        let straight3 = vec![
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Seven),
        ];
        let straight4 = vec![
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Seven),
            Card::new(Suit::random(), Rank::Eight),
        ];
        let straight5 = vec![
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Seven),
            Card::new(Suit::random(), Rank::Eight),
            Card::new(Suit::random(), Rank::Nine),
        ];
        let straight6 = vec![
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Seven),
            Card::new(Suit::random(), Rank::Eight),
            Card::new(Suit::random(), Rank::Nine),
            Card::new(Suit::random(), Rank::Ten),
        ];
        let straight7 = vec![
            Card::new(Suit::random(), Rank::Seven),
            Card::new(Suit::random(), Rank::Eight),
            Card::new(Suit::random(), Rank::Nine),
            Card::new(Suit::random(), Rank::Ten),
            Card::new(Suit::random(), Rank::Jack),
        ];
        let straight8 = vec![
            Card::new(Suit::random(), Rank::Eight),
            Card::new(Suit::random(), Rank::Nine),
            Card::new(Suit::random(), Rank::Ten),
            Card::new(Suit::random(), Rank::Jack),
            Card::new(Suit::random(), Rank::Queen),
        ];
        let straight9 = vec![
            Card::new(Suit::random(), Rank::Nine),
            Card::new(Suit::random(), Rank::Ten),
            Card::new(Suit::random(), Rank::Jack),
            Card::new(Suit::random(), Rank::Queen),
            Card::new(Suit::random(), Rank::King),
        ];
        let straight10 = vec![
            Card::new(Suit::random(), Rank::Ten),
            Card::new(Suit::random(), Rank::Jack),
            Card::new(Suit::random(), Rank::Queen),
            Card::new(Suit::random(), Rank::King),
            Card::new(Suit::random(), Rank::Ace),
        ];
        // assert the evaluation of each one is greater than the last
        assert!(evaluate_straight(&straight1) < evaluate_straight(&straight2));
        assert!(evaluate_straight(&straight2) < evaluate_straight(&straight3));
        assert!(evaluate_straight(&straight3) < evaluate_straight(&straight4));
        assert!(evaluate_straight(&straight4) < evaluate_straight(&straight5));
        assert!(evaluate_straight(&straight5) < evaluate_straight(&straight6));
        assert!(evaluate_straight(&straight6) < evaluate_straight(&straight7));
        assert!(evaluate_straight(&straight7) < evaluate_straight(&straight8));
        assert!(evaluate_straight(&straight8) < evaluate_straight(&straight9));
        assert!(evaluate_straight(&straight9) < evaluate_straight(&straight10));
    }
}

pub fn generate_unique_fives(
    lower_take_index: usize,
    upper_take_index: usize,
) -> [u16; BIT_REP_LIMIT + 1] {
    let mut lower_take_set = HashMap::<u16, u32>::new();
    let mut upper_take_set = HashMap::<u16, u32>::new();

    for hand in generate_all_unique_rank_combos(5) {
        let hand_id = hand_to_id(&hand);
        let rank_mask = unique_rank_mask(&hand_id) as u16;

        match is_straight(&hand) {
            false => {
                if lower_take_set.contains_key(&rank_mask) {
                    panic!("Duplicate entry found in lower_take_set: {:?}", hand);
                }
                lower_take_set.insert(rank_mask, rank_mask as u32)
            }
            true => {
                if upper_take_set.contains_key(&rank_mask) {
                    panic!("Duplicate entry found in upper_take_set: {:?}", hand);
                }
                upper_take_set.insert(rank_mask, evaluate_straight(&hand))
            }
        };
    }

    assert!(lower_take_set.len() == 1277);
    assert!(upper_take_set.len() == 10);

    let mut sorted_lower: Vec<_> = lower_take_set.iter().collect();
    sorted_lower.sort_by_key(|&(_, v)| v);

    let mut sorted_upper: Vec<_> = upper_take_set.iter().collect();
    sorted_upper.sort_by_key(|&(_, v)| v);

    let mut return_flushes = [0u16; BIT_REP_LIMIT + 1];

    for (index, (rank_mask, _)) in sorted_lower.iter().enumerate().take(1277) {
        return_flushes[(**rank_mask) as usize] = (index + lower_take_index) as u16;
    }

    for (index, (rank_mask, _)) in sorted_upper.iter().enumerate().take(10) {
        return_flushes[(**rank_mask) as usize] = (index + upper_take_index) as u16;
    }
    return_flushes
}

pub fn generate_flushes_table() -> [u16; BIT_REP_LIMIT + 1] {
    let lower_take_index = 5863 + 1; // 7937 - 322 - 10, 322 == 156 quads + 156 full houses + 10 straight flushes
    let upper_take_index = 7452 + 1; // 7937 - 10, 10 straight flushes
    generate_unique_fives(lower_take_index, upper_take_index)
}

#[cfg(test)]
mod flush_tests {
    use crate::evaluate::evaluate_hand::{id_mask_to_string, DISTINCT_CARD_COMBOS};

    use super::*;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref FLUSHES_MAP: [u16; BIT_REP_LIMIT + 1] = generate_flushes_table();
    }

    fn evaluate_flush(hand: &[Card]) -> u16 {
        let hand = hand_to_id(hand);
        let rank_mask = unique_rank_mask(&hand);
        
        FLUSHES_MAP[rank_mask as usize]
    }

    // A test to show the hand evaluation and board evaluation order doesn't mattter
    #[test]
    fn order_invariance_hand() {
        for _ in 0..1_000 {
            let hand = Card::new_random_cards(5);
            let first_eval = evaluate_flush(&hand);
            for perm in hand.into_iter().permutations(5) {
                assert_eq!(first_eval, evaluate_flush(&perm));
            }
        }
    }

    fn compare_flushes(hand1: &Vec<Card>, hand2: &Vec<Card>, ord: std::cmp::Ordering) {
        if hand1.iter().unique().count() != 5 || hand2.iter().unique().count() != 5 {
            panic!("Hands must have 5 unique cards");
        };

        let evaluation1 = evaluate_flush(hand1);
        let evaluation2 = evaluate_flush(hand2);

        match ord {
            std::cmp::Ordering::Less => assert!(evaluation1 < evaluation2),
            std::cmp::Ordering::Equal => assert!(evaluation1 == evaluation2),
            std::cmp::Ordering::Greater => assert!(evaluation1 > evaluation2),
        }
    }

    #[test]
    fn assert_rankings_are_exact() {
        let mut seen_rankings = vec![0; BIT_REP_LIMIT + 1];
        let mut count = 0;
        for ranking in FLUSHES_MAP.iter() {
            if ranking == &0 {
                continue;
            }
            count += 1;
            if seen_rankings[*ranking as usize] != 0 {
                panic!(
                    "Flush table has duplicate entry {}",
                    id_mask_to_string((*ranking as u32) << 12)
                );
            }
            seen_rankings[*ranking as usize] += 1;
        }
        assert_eq!(count, STRAIGHT_COUNT + NON_STRAIGHT_COUNT);
    }

    #[test]
    fn test_straight_flushes() {
        let hand0 = vec![
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
        ];

        let hand1 = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
        ];

        let hand2 = vec![
            Card::new(Suit::random(), Rank::Ten),
            Card::new(Suit::random(), Rank::Jack),
            Card::new(Suit::random(), Rank::Queen),
            Card::new(Suit::random(), Rank::King),
            Card::new(Suit::random(), Rank::Ace),
        ];

        let eval0 = evaluate_flush(&hand0);
        let eval1 = evaluate_flush(&hand1);
        let eval2 = evaluate_flush(&hand2);

        println!("Eval0: {}, Eval1: {}, Eval2: {}", eval0, eval1, eval2);

        compare_flushes(&hand1, &hand2, std::cmp::Ordering::Less);
        compare_flushes(&hand0, &hand1, std::cmp::Ordering::Less);
    }

    #[test]
    fn test_straight_flushes_2() {
        let hand1 = vec![
            Card::new(Suit::random(), Rank::Nine),
            Card::new(Suit::random(), Rank::Ten),
            Card::new(Suit::random(), Rank::Jack),
            Card::new(Suit::random(), Rank::Queen),
            Card::new(Suit::random(), Rank::King),
        ];

        let hand2 = vec![
            Card::new(Suit::random(), Rank::Ten),
            Card::new(Suit::random(), Rank::Jack),
            Card::new(Suit::random(), Rank::Queen),
            Card::new(Suit::random(), Rank::King),
            Card::new(Suit::random(), Rank::Ace),
        ];

        compare_flushes(&hand1, &hand2, std::cmp::Ordering::Less);
    }

    #[test]
    fn test_flush_1() {
        let hand1 = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Eight),
            Card::new(Suit::random(), Rank::Nine),
        ];

        let hand2 = vec![
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Seven),
            Card::new(Suit::random(), Rank::Ten),
            Card::new(Suit::random(), Rank::Jack),
        ];

        compare_flushes(&hand1, &hand2, std::cmp::Ordering::Less);
    }

    #[test]
    fn test_equal_straight_flushes() {
        let hand1 = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
        ];

        let hand2 = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
        ];

        compare_flushes(&hand1, &hand2, std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_equal_flushes() {
        let hand1 = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Eight),
            Card::new(Suit::random(), Rank::Ten),
        ];

        let hand2 = vec![
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Ten),
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Eight),
            Card::new(Suit::random(), Rank::Six),
        ];

        compare_flushes(&hand1, &hand2, std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_worst_flush() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Seven),
        ];
        let eval = evaluate_flush(&hand);
        assert_eq!(eval, 5864);
    }

    #[test]
    fn test_low_card_diff() {
        let hand1 = vec![
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Seven),
            Card::new(Suit::random(), Rank::Eight),
        ];

        let hand2 = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Seven),
            Card::new(Suit::random(), Rank::Eight),
        ];

        compare_flushes(&hand1, &hand2, std::cmp::Ordering::Greater);
    }

    #[test]
    fn test_best_hand() {
        let royal_flush = vec![
            Card::new(Suit::random(), Rank::Ace),
            Card::new(Suit::random(), Rank::Ten),
            Card::new(Suit::random(), Rank::Jack),
            Card::new(Suit::random(), Rank::Queen),
            Card::new(Suit::random(), Rank::King),
        ];
        let eval = evaluate_flush(&royal_flush);

        println!("Royal flush: {}", eval);
        assert_eq!(eval, DISTINCT_CARD_COMBOS as u16);
    }
}
