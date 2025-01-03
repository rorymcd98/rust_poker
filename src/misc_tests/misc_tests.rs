use itertools::Itertools;

use crate::models::card::Rank;

use crate::models::Card;
use crate::traversal::{main_train::get_unique_cards, strategy::strategy_branch::StrategyHubKey};

#[test]
fn test_get_unique_cards() {
    for _ in 0..10_000 {
        let deck = Card::shuffle_deck();
        for i in 0..48 {
            let key1 = StrategyHubKey {
                low_rank: deck[i].rank, // These aren't sorted like they are usually but that doesn't matter
                high_rank: deck[i + 1].rank,
                is_suited: deck[i].suit == deck[i + 1].suit,
                is_sb: true,
            };
            let key2 = StrategyHubKey {
                low_rank: deck[i].rank,
                high_rank: deck[i + 1].rank,
                is_suited: deck[i].suit == deck[i + 1].suit,
                is_sb: false,
            };

            let cards = get_unique_cards(&key1, &key2);
            assert_eq!(cards.iter().unique().count(), 4);
            if key1.is_suited {
                assert_eq!(cards[0].suit, cards[1].suit);
            } else {
                assert_ne!(cards[0].suit, cards[1].suit);
            }

            if key2.is_suited {
                assert_eq!(cards[2].suit, cards[3].suit);
            } else {
                assert_ne!(cards[2].suit, cards[3].suit);
            }
        }
    }
}