use generate_flush_table::{generate_all_unique_rank_combos, generate_flushes_table};
use generate_remaining_table::generate_remaining_table;
use generate_unique_five_table::generate_unique_five_table;

use crate::evaluate::evaluate_hand::{id_to_card_string, DISTINCT_COUNT};

use super::*;

#[test]
fn test_mutual_exclusivity() {
    let flush_table = generate_flushes_table();
    let five_table = generate_unique_five_table();
    let remaining_table = generate_remaining_table();
    
    let mut seen_rankings = vec![0; DISTINCT_COUNT + 1];
    for i in flush_table.iter() {
        if seen_rankings[*i as usize] != 0 {
            panic!("Flush table has duplicate entries {}", id_to_card_string(*i as u32));
        }
        seen_rankings[*i as usize] += 1;
    }
}