use generate_flush_table::generate_flushes_table;
use generate_remaining_table::generate_remaining_table;
use generate_unique_five_table::generate_unique_five_table;

use crate::evaluate::evaluate_hand::{id_mask_to_string, prime_product_to_rank_string, DISTINCT_CARD_COMBOS};

use super::*;

#[test]
fn test_mutual_exclusivity() {
    let flush_table = generate_flushes_table();
    let five_table = generate_unique_five_table();

    let mut count = 0;
    let mut seen_rankings = vec![0; DISTINCT_CARD_COMBOS + 1];
    for (id, ranking) in flush_table.iter().enumerate() {
        if *ranking == 0 {
            continue;
        }
        count += 1;
        if seen_rankings[*ranking as usize] != 0 {
            panic!("Flush table has duplicate entries {}", id_mask_to_string((id as u32) << 12));
        }
        seen_rankings[*ranking as usize] += 1;
    }

    for id in five_table.iter() {
        if *id == 0 {
            continue;
        }
        count += 1;
        if seen_rankings[*id as usize] != 0 {
            panic!("Fives table has duplicate entries {}", id_mask_to_string((*id as u32) << 12));
        }
        seen_rankings[*id as usize] += 1;
    }

    let remaining_table = generate_remaining_table();

    for (prime_product, ranking) in remaining_table.iter().enumerate() {
        if *ranking == 0 {
            continue;
        }
        count += 1;
        if seen_rankings[*ranking as usize] != 0 {
            panic!("Remaining table has duplicate entries {}, conflicts with rank {}", prime_product_to_rank_string(prime_product), ranking);
        }
        seen_rankings[*ranking as usize] += 1;
}

    assert_eq!(count, DISTINCT_CARD_COMBOS);
}