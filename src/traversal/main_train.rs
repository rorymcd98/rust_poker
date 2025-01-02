use rand::seq::SliceRandom;

use crate::config::*;
use super::game_state::game_state_helper::GameStateHelper;
use super::game_state::terminal_state::TerminalState;
use super::strategy;
use super::strategy::strategy_trait::Strategy;
use super::strategy::training_strategy::{sample_strategy, TrainingStrategy};
use super::strategy::strategy_branch::{StrategyBranch, StrategyHubElement};
use super::strategy::strategy_hub::{StrategyHub, StrategyHubBucket};
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
    let all_rank_combos = all_rank_combos();
    let mut sb_elements = all_rank_combos
        .iter()
        .map(|(low, high)| StrategyHubElement{low_rank: *low, high_rank: *high, is_sb: true, is_suited: true})
        .collect::<Vec<StrategyHubElement>>();
    sb_elements.extend(
        all_rank_combos
            .iter()
            .map(|(low, high)| StrategyHubElement{low_rank: *low, high_rank: *high, is_sb: true, is_suited: false})
            .collect::<Vec<StrategyHubElement>>(),
    );
    sb_elements.extend(
        all_pocket_pairs().iter().map(|rank| StrategyHubElement{low_rank: rank.0, high_rank: rank.1, is_sb: true, is_suited: false})
    );

    let bb_elements = sb_elements.clone().into_iter().map(|element| StrategyHubElement{is_sb: false, ..element}).collect::<Vec<StrategyHubElement>>();

    let strategy_hub = StrategyHub::new(bb_elements.len(), STRATEGY_HUB_TAKE, STRATEGY_HUB_RESERVE);
    with_rng(|rng| {
        let mut sb_strategies: Vec<StrategyBranch<TrainingStrategy>> = sb_elements.iter().map(|element| StrategyBranch::new(element.clone())).collect();
        let mut bb_strategies: Vec<StrategyBranch<TrainingStrategy>> = bb_elements.iter().map(|element| StrategyBranch::new(element.clone())).collect();
        sb_strategies.shuffle(rng);
        bb_strategies.shuffle(rng);
        strategy_hub.return_elements(StrategyHubBucket {
            sb_strategies,
            bb_strategies,
        });
    });

    let strategy_hub = Arc::new(strategy_hub);
        
    let mut handles = vec![];
    for _ in 0..NUM_THREADS {
        let strategy_hub_clone = Arc::clone(&strategy_hub);
        handles.push(spawn_training_thread_work(strategy_hub_clone));
    }

    for handle in handles {
        let _ = handle.join().unwrap();
    }

    let map = Arc::try_unwrap(strategy_hub).expect("Arc has more than one strong reference").into_map();

    // for s in sb_elements{
    //     println!("{:?},", s);
    //     map.get(&s).unwrap().print_stats();
    // }

    // for s in bb_elements{
    //     println!("{:?}", s);
    //     map.get(&s).unwrap().print_stats();
    // }
}

fn spawn_training_thread_work(strategy_hub: Arc<StrategyHub<TrainingStrategy>>) -> JoinHandle<()> {
    fn second_suit(element: &StrategyBranch<TrainingStrategy>) -> Suit {
        if element.strategy_hub_element.is_suited {
            Suit::Spades
        } else {
            Suit::Clubs
        }
    }

    thread::spawn(move || {
        let players = [Player::Traverser, Player::Opponent];
        // get all combos of sb_elements and bb_elements
        for i in 1..TRAIN_ITERATIONS {
            println!("Training iteration {}", i);
            let training_bucket = strategy_hub.get_more_elements();
            let sb_bucket_cell = RefCell::new(training_bucket.sb_strategies);
            let bb_bucket_cell = RefCell::new(training_bucket.bb_strategies); 

            for sb_branch in &mut sb_bucket_cell.borrow_mut().iter_mut() {
                for bb_branch in &mut bb_bucket_cell.borrow_mut().iter_mut() {
                    for player in players {
                        let deal = match player {
                            Player::Traverser => Card::new_random_9_card_game_with(
                                Card::new(Suit::Spades, sb_branch.strategy_hub_element.low_rank.clone()),
                                Card::new(second_suit(&sb_branch), sb_branch.strategy_hub_element.high_rank.clone()),
                                Card::new(Suit::Spades, bb_branch.strategy_hub_element.low_rank),
                                Card::new(second_suit(&bb_branch), bb_branch.strategy_hub_element.high_rank),
                            ),
                            Player::Opponent => Card::new_random_9_card_game_with(
                                Card::new(Suit::Spades, bb_branch.strategy_hub_element.low_rank),
                                Card::new(second_suit(&bb_branch), bb_branch.strategy_hub_element.high_rank.clone()),
                                Card::new(Suit::Spades, sb_branch.strategy_hub_element.low_rank),
                                Card::new(second_suit(&sb_branch), sb_branch.strategy_hub_element.high_rank.clone()),
                            ),
                        };

                        let mut branch_traverser = TrainingBranchTraverser::new(sb_branch, bb_branch, GameStateHelper::new(deal, player), i);
                        branch_traverser.begin_traversal();
                    }
                }
            }
            strategy_hub.return_elements(StrategyHubBucket {
                sb_strategies: sb_bucket_cell.into_inner(),
                bb_strategies: bb_bucket_cell.into_inner(),
            });
        }
    })
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
            // log the current strategy
            // if self.training_iteration > 100 && self.game_state.is_river(){
            //     let our_cards = self.game_state.get_current_player_cards();
            //     let opp_cards = self.game_state.get_non_current_player_cards();
            //     println!("Strategy for {}{} vs {}{}: {:?}", our_cards[0], our_cards[1], opp_cards[0], opp_cards[1], current_strategy);
            //     println!("strategy updates {} .", strategy.updates);
            // }        
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