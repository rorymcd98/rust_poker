use rand::seq::SliceRandom;
use rand::Rng;

use crate::config::*;
use super::game_state::game_state_helper::GameStateHelper;
use super::game_state::terminal_state::TerminalState;
use super::strategy::strategy_trait::Strategy;
use super::strategy::training_strategy::{sample_strategy, TrainingStrategy};
use super::strategy::strategy_branch::{StrategyBranch, StrategyHubKey};
use super::strategy::strategy_hub::{deserialise_strategy_hub, serialise_strategy_hub, StrategyHub, StrategyPair};
use crate::models::card::{all_pocket_pairs, all_rank_combos};
use crate::models::Card;
use crate::models::Player;
use crate::models::Suit;
use crate::thread_utils::with_rng;
use std::cell::RefCell;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

pub fn begin_tree_train_traversal() {
    let strategy_hub = load_or_create_strategy_hub();
    let strategy_hub = Arc::new(strategy_hub);
        
    let mut handles = vec![];
    println!("Training with {} threads", NUM_THREADS);
    for _ in 0..NUM_THREADS {
        let strategy_hub_clone = Arc::clone(&strategy_hub);
        handles.push(spawn_training_thread_work(strategy_hub_clone));
    }

    for handle in handles {
        let _ = handle.join().unwrap();
    }

    let strategy_hub = Arc::try_unwrap(strategy_hub).expect("Arc has more than one strong reference");
    serialise_strategy_hub(BLUEPRINT_FOLDER, strategy_hub).expect("Failed to serialise strategy hub");
}

pub fn get_unique_cards(sb_key: &StrategyHubKey, bb_key: &StrategyHubKey) -> [Card; 4] {
    let card1 = Card::new(Suit::Spades, sb_key.low_rank);
    let mut card2 = Card::new(Suit::Spades, sb_key.high_rank);
    let mut card3 = Card::new(Suit::Spades, bb_key.low_rank);
    let mut card4 = Card::new(Suit::Spades, bb_key.high_rank);
    
    if !sb_key.is_suited {
        card2.suit = Suit::Clubs; 
    }

    with_rng(|rng| {
        if card3 == card1 || card3 == card2 || rng.gen_bool(0.75) { // TODO - I'm not sure if these two 0.75 probabilities are correct but it is probabbly fine
            card3.suit = Suit::Hearts;
        }
        if bb_key.is_suited {
            card4.suit = card3.suit;
            if card4 == card1 || card4 == card2 {
                card3.suit = Suit::Hearts;
                card4.suit = Suit::Hearts;
            }
        } else {
            if card4 == card1 || card4 == card2 || card4 == card3 || rng.gen_bool(0.75) {
                card4.suit = Suit::Diamonds;
            }
        }
    });
    [card1, card2, card3, card4]
}

fn spawn_training_thread_work(strategy_hub: Arc<StrategyHub<TrainingStrategy>>) -> JoinHandle<()> {
    thread::spawn(move || {
        let players = [Player::Traverser, Player::Opponent];
        // get all combos of sb_elements and bb_elements
        for i in 1..TRAIN_ITERATIONS {
            let training_bucket = strategy_hub.get_more_elements();
            let mut sb_branch = training_bucket.sb_branch;
            let mut bb_branch = training_bucket.bb_branch;

            let cards = get_unique_cards(&sb_branch.strategy_hub_key, &bb_branch.strategy_hub_key);

            for player in players {
                let mut deal = match player {
                    Player::Traverser => Card::new_random_nine_card_game_with(
                        cards[0],
                        cards[1],
                        cards[2],
                        cards[3],
                    ),
                    Player::Opponent => Card::new_random_nine_card_game_with(
                        cards[2],
                        cards[3],
                        cards[0],
                        cards[1],
                    ),
                };
                let mut branch_traverser = TrainingBranchTraverser::new(&mut sb_branch, &mut bb_branch, GameStateHelper::new(deal, player), i);
                branch_traverser.begin_traversal();
            }

            strategy_hub.return_strategies(StrategyPair {
                sb_branch,
                bb_branch,
            });
        }
    })
}

fn load_or_create_strategy_hub() -> StrategyHub<TrainingStrategy> {
    deserialise_strategy_hub(BLUEPRINT_FOLDER).unwrap_or_else(|err| {
        println!("Could not deserialise an existing strategy-hub, creating new strategy hub: {}", err);
        create_new_all_cards_strategy_hub()
    })
}

fn create_new_all_cards_strategy_hub() -> StrategyHub<TrainingStrategy> {
    let all_rank_combos = all_rank_combos();
    let mut sb_elements = all_rank_combos
        .iter()
        .map(|(low, high)| StrategyHubKey{low_rank: *low, high_rank: *high, is_sb: true, is_suited: true})
        .collect::<Vec<StrategyHubKey>>();
    sb_elements.extend(
        all_rank_combos
            .iter()
            .map(|(low, high)| StrategyHubKey{low_rank: *low, high_rank: *high, is_sb: true, is_suited: false})
            .collect::<Vec<StrategyHubKey>>(),
    );
    sb_elements.extend(
        all_pocket_pairs().iter().map(|rank| StrategyHubKey{low_rank: rank.0, high_rank: rank.1, is_sb: true, is_suited: false})
    );

    let bb_elements = sb_elements.clone().into_iter().map(|element| StrategyHubKey{is_sb: false, ..element}).collect::<Vec<StrategyHubKey>>();

    let strategy_hub = StrategyHub::new(
        bb_elements
            .into_iter()
            .chain(sb_elements.into_iter())
            .map(|key| StrategyBranch::new(key))
            .collect()
    );
    strategy_hub
}

struct TrainingBranchTraverser<'a> {
    training_iteration: usize,
    game_state: GameStateHelper,
    sb_strategy_branch: &'a mut StrategyBranch<TrainingStrategy>,
    bb_strategy_branch: &'a mut StrategyBranch<TrainingStrategy>,
}

impl<'a> TrainingBranchTraverser<'a> {
    pub fn new(
        sb_strategy_branch: &'a mut StrategyBranch<TrainingStrategy>,
        bb_strategy_branch: &'a mut StrategyBranch<TrainingStrategy>,
        game_state: GameStateHelper,
        training_iteration: usize,
    ) -> TrainingBranchTraverser<'a> {
        TrainingBranchTraverser {
            sb_strategy_branch,
            bb_strategy_branch,
            training_iteration,
            game_state,
        }
    }

    pub fn begin_traversal(&mut self) -> f32 {
        self.traverse_action()
    }

    fn traverse_action(&mut self) -> f32 {
        match self.game_state.check_round_terminal() {
            TerminalState::Showdown => return self.game_state.evaluate_showdown(),
            TerminalState::Fold => return self.game_state.evaluate_fold(),
            TerminalState::RoundOver => {
                if self.game_state.is_preflop() {
                    return self.traverse_flop();
                }
                return self.traverse_deal();
            }
            TerminalState::None => (),
        };

        let num_available_actions = self.game_state.get_num_available_actions();

        let pot_before_action = self.game_state.get_current_player_pot();
        let bets_before_action = self.game_state.bets_this_round.get();
        let previous_player = self.game_state.current_player.get();
        let checks_before = self.game_state.checks_this_round.get();

        let training_iteration = self.training_iteration;

        let strategy = self.get_strategy();
        let current_strategy = strategy.get_current_strategy(training_iteration);

        if self.game_state.current_player.get().is_opponent() {
            let sampled_action = sample_strategy(&current_strategy, num_available_actions);
            self.traverse_chosen_action(sampled_action, previous_player, pot_before_action, bets_before_action, checks_before)
        } else {
            let mut utility = 0.0;
            let mut utilities = vec![0.0; num_available_actions];
            for action in 0..num_available_actions {
                utilities[action] = self.traverse_chosen_action(action, previous_player, pot_before_action, bets_before_action, checks_before);
                utility += utilities[action] * current_strategy[action];
            }

            let strategy = self.get_strategy();    
            strategy.update_strategy(utility, &utilities, training_iteration);   
            utility
        }
    }

    fn traverse_chosen_action(&mut self, action: usize, acting_player: Player, current_pot: u8, current_bets: u8, current_checks: u8) -> f32 {
        match action {
            0 => self.game_state.checkfold(),
            1 => self.game_state.call_or_bet(),
            2 => self.game_state.bet(),
            _ => panic!("Invalid action"),
        };
        self.game_state.switch_current_player();
        let utility = self.traverse_action();
        self.game_state.undo(
            acting_player,
            current_pot,
            current_bets,
            current_checks,
        );
        utility
    }

    fn traverse_flop(&mut self) -> f32 {
        let previous_player = self.game_state.current_player.get();
        let previous_bets = self.game_state.bets_this_round.get();
        let check_before = self.game_state.checks_this_round.get();
        self.game_state.deal_flop();
        let utility = self.traverse_action();
        self.game_state
            .undeal_flop(previous_bets, previous_player, check_before);
        utility
    }

    fn traverse_deal(&mut self) -> f32 {
        let previous_player = self.game_state.current_player.get();
        let previous_bets = self.game_state.bets_this_round.get();
        let checks_before = self.game_state.checks_this_round.get();
        self.game_state.deal();
        let utility = self.traverse_action();
        self.game_state
            .undeal(previous_bets, previous_player, checks_before);
        utility
    }

    fn get_strategy(&mut self) -> &mut TrainingStrategy {
        let strategy_branch = if self.game_state.current_player.get() == self.game_state.small_blind_player {
            &mut self.sb_strategy_branch
        } else {
            &mut self.bb_strategy_branch
        };
        strategy_branch.get_or_create_strategy(self.game_state.serialise_history_with_current_player(), self.game_state.get_num_available_actions())
    }
}