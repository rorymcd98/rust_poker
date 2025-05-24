use super::generate_flush_table::generate_unique_fives;
use crate::evaluate::evaluate_hand::BIT_REP_LIMIT;

/// Generate a lookup table for unique five card hands that aren't flushes
pub fn generate_unique_five_table() -> [u16; BIT_REP_LIMIT + 1] {
    const LOWER_TAKE_INDEX: usize = 0;
    const UPPER_TAKE_INDEX: usize = 5853; // 1277 high cards + 2860 pairs + 858 two-pair + 858 ToaK
    generate_unique_fives(LOWER_TAKE_INDEX + 1, UPPER_TAKE_INDEX + 1)
}

#[cfg(test)]
mod tests {
    use crate::{
        evaluate::{
            evaluate_hand::{hand_to_id, id_mask_to_string, unique_rank_mask},
            generate_tables::generate_flush_table::{NON_STRAIGHT_COUNT, STRAIGHT_COUNT},
        },
        models::card::Card,
        models::card::Rank,
        models::card::Suit,
    };

    use super::*;
    use itertools::Itertools;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref FIVE_UNIQUES_MAP: [u16; BIT_REP_LIMIT + 1] = generate_unique_five_table();
    }

    fn evaluate_fives(hand: &[Card]) -> u16 {
        let hand = hand_to_id(hand);
        let rank_mask = unique_rank_mask(&hand);
        
        FIVE_UNIQUES_MAP[rank_mask as usize]
    }

    // A test to show the hand evaluation and board evaluation order doesn't mattter
    #[test]
    fn order_invariance_hand() {
        for _ in 0..1_000 {
            let hand = Card::new_random_cards(5);
            let first_eval = evaluate_fives(&hand);
            for perm in hand.into_iter().permutations(5) {
                assert_eq!(first_eval, evaluate_fives(&perm));
            }
        }
    }

    fn compare_fives(hand1: &Vec<Card>, hand2: &[Card], ord: std::cmp::Ordering) {
        if hand1.iter().unique().count() != 5 || hand2.iter().unique().count() != 5 {
            panic!("Hands must have 5 unique cards");
        };

        let evaluation1 = evaluate_fives(hand1);
        let evaluation2 = evaluate_fives(hand2);

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
        for ranking in FIVE_UNIQUES_MAP.iter() {
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
    fn worst_cards() {
        let hand = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Seven),
        ];
        let evaluation = evaluate_fives(&hand);
        assert_eq!(evaluation, 1);

        let hand = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Seven),
        ];
        let evaluation = evaluate_fives(&hand);
        assert_eq!(evaluation, 2);

        let hand = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Seven),
        ];
        let evaluation = evaluate_fives(&hand);
        assert_eq!(evaluation, 3);

        let hand = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Seven),
        ];
        let evaluation = evaluate_fives(&hand);
        assert_eq!(evaluation, 4);
    }

    #[test]
    fn high_card_comparisons() {
        let hand1 = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Eight),
        ];
        let hand2 = vec![
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Seven),
            Card::new(Suit::random(), Rank::Nine),
        ];
        compare_fives(&hand1, &hand2, std::cmp::Ordering::Less);

        let hand3 = vec![
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Seven),
            Card::new(Suit::random(), Rank::Eight),
            Card::new(Suit::random(), Rank::Ten),
        ];
        compare_fives(&hand2, &hand3, std::cmp::Ordering::Less);

        let hand4 = vec![
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Seven),
            Card::new(Suit::random(), Rank::Eight),
            Card::new(Suit::random(), Rank::Nine),
            Card::new(Suit::random(), Rank::Jack),
        ];
        compare_fives(&hand3, &hand4, std::cmp::Ordering::Less);
    }

    #[test]
    fn straight_comparisons() {
        let hand1 = vec![
            Card::new(Suit::random(), Rank::Two),
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
        ];
        let hand2 = vec![
            Card::new(Suit::random(), Rank::Three),
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Seven),
        ];
        compare_fives(&hand1, &hand2, std::cmp::Ordering::Less);

        let hand3 = vec![
            Card::new(Suit::random(), Rank::Four),
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Seven),
            Card::new(Suit::random(), Rank::Eight),
        ];
        compare_fives(&hand2, &hand3, std::cmp::Ordering::Less);

        let hand4 = vec![
            Card::new(Suit::random(), Rank::Five),
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Seven),
            Card::new(Suit::random(), Rank::Eight),
            Card::new(Suit::random(), Rank::Nine),
        ];
        compare_fives(&hand3, &hand4, std::cmp::Ordering::Less);

        let hand5 = vec![
            Card::new(Suit::random(), Rank::Six),
            Card::new(Suit::random(), Rank::Seven),
            Card::new(Suit::random(), Rank::Eight),
            Card::new(Suit::random(), Rank::Nine),
            Card::new(Suit::random(), Rank::Ten),
        ];
        compare_fives(&hand4, &hand5, std::cmp::Ordering::Less);

        let hand6 = vec![
            Card::new(Suit::random(), Rank::Seven),
            Card::new(Suit::random(), Rank::Eight),
            Card::new(Suit::random(), Rank::Nine),
            Card::new(Suit::random(), Rank::Ten),
            Card::new(Suit::random(), Rank::Jack),
        ];
        compare_fives(&hand5, &hand6, std::cmp::Ordering::Less);
    }
}
