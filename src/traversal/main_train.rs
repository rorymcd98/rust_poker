use rand::seq::SliceRandom;
use rand::Rng;
use crate::config::*;
use super::game_state::game_state_helper::GameStateHelper;
use super::game_state::terminal_state::TerminalState;
use super::strategy::strategy_trait::Strategy;
use super::strategy::training_strategy::{sample_strategy, TrainingStrategy};
use super::strategy::strategy_branch::{StrategyBranch, StrategyHubKey};
use super::strategy::strategy_hub::{deserialise_strategy_hub, serialise_strategy_hub, StrategyHub, StrategyPair};
use crate::models::card::{all_pocket_pairs, all_rank_combos, new_random_nine_card_game_with};
use crate::models::Card;
use crate::models::Player;
use crate::models::Suit;
use crate::thread_utils::with_rng;
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

    let strategy_hub: StrategyHub<TrainingStrategy> = Arc::try_unwrap(strategy_hub).expect("Arc has more than one strong reference");
    // validate_strategy_map::<TrainingStrategy>(&strategy_hub.into_map());
    serialise_strategy_hub(BLUEPRINT_FOLDER, strategy_hub).expect("Failed to serialise strategy hub");
}

// Get unique cards ensuring that there's no overlap between the two hands, and no sampling bias
pub fn get_unique_cards(sb_key: &StrategyHubKey, bb_key: &StrategyHubKey) -> [Card; 4] {
    // First two cards (SB hand)
    let card1 = Card::new(Suit::Spades, sb_key.low_rank);
    let card2 = if sb_key.is_suited {
        Card::new(Suit::Spades, sb_key.high_rank)
    } else {
        Card::new(Suit::Clubs, sb_key.high_rank)
    };
    
    let sb_cards = [card1, card2];
    
    // Generate all valid combinations for BB hand
    let mut valid_combinations = Vec::new();
    
    if bb_key.is_suited {
        // BB hand is suited - both cards must have same suit
        for suit in [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs] {
            let card3 = Card::new(suit, bb_key.low_rank);
            let card4 = Card::new(suit, bb_key.high_rank);
            
            // Check if this combination conflicts with SB cards
            if !sb_cards.contains(&card3) && !sb_cards.contains(&card4) {
                valid_combinations.push([card3, card4]);
            }
        }
    } else {
        // BB hand is offsuit - cards can have different suits
        for suit3 in [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs] {
            for suit4 in [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs] {
                // Skip if both cards would have the same suit (since it's offsuit)
                if suit3 == suit4 {
                    continue;
                }
                
                let card3 = Card::new(suit3, bb_key.low_rank);
                let card4 = Card::new(suit4, bb_key.high_rank);
                
                // Check if this combination conflicts with SB cards
                if !sb_cards.contains(&card3) && !sb_cards.contains(&card4) {
                    valid_combinations.push([card3, card4]);
                }
            }
        }
    }
    
    // Sample uniformly from valid combinations
    with_rng(|rng| {
        let bb_cards = valid_combinations.choose(rng)
            .expect("No valid card combinations found");
        
        [card1, card2, bb_cards[0], bb_cards[1]]
    })
}

fn spawn_training_thread_work(strategy_hub: Arc<StrategyHub<TrainingStrategy>>) -> JoinHandle<()> {
    thread::spawn(move || {
        let players = [Player::Traverser, Player::Opponent];
        // get all combos of sb_elements and bb_elements
        for i in 1..TRAIN_ITERATIONS {
            if i % ITERATION_UPDATES == 0 {
                println!("Training iteration: {}/{}", i, TRAIN_ITERATIONS);
            } 
            let training_bucket = strategy_hub.get_more_elements();
            let mut sb_branch = training_bucket.sb_branch;
            let mut bb_branch = training_bucket.bb_branch;

            let cards = get_unique_cards(&sb_branch.strategy_hub_key, &bb_branch.strategy_hub_key);

            for player in players {
                let deal = match player {
                    Player::Traverser => new_random_nine_card_game_with(
                        cards[0],
                        cards[1],
                        cards[2],
                        cards[3],
                    ),
                    Player::Opponent => new_random_nine_card_game_with(
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
    match deserialise_strategy_hub(BLUEPRINT_FOLDER) {
        Ok(strategy_hub) => StrategyHub::from_map(strategy_hub).unwrap(),
        Err(err) => {
            println!("Could not deserialise an existing strategy-hub, creating new strategy hub: {}", err);
            create_new_all_cards_strategy_hub()
        }
    }
}

pub fn get_all_combos_by_blind(is_smallblind: bool) -> Vec<StrategyHubKey> {
    let all_rank_combos = all_rank_combos();
    let mut sb_elements = all_rank_combos
        .iter()
        .map(|(low, high)| StrategyHubKey{low_rank: *low, high_rank: *high, is_sb: is_smallblind, is_suited: true})
        .collect::<Vec<StrategyHubKey>>();
    sb_elements.extend(
        all_rank_combos
            .iter()
            .map(|(low, high)| StrategyHubKey{low_rank: *low, high_rank: *high, is_sb: is_smallblind, is_suited: false})
            .collect::<Vec<StrategyHubKey>>(),
    );
    sb_elements.extend(
        all_pocket_pairs().iter().map(|rank| StrategyHubKey{low_rank: rank.0, high_rank: rank.1, is_sb: is_smallblind, is_suited: false})
    );
    sb_elements
}

fn create_new_all_cards_strategy_hub() -> StrategyHub<TrainingStrategy> {
    let sb_elements = get_all_combos_by_blind(true);
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

    pub fn begin_traversal(&mut self) -> f64 {
        self.traverse_action()
    }

    fn traverse_action(&mut self) -> f64 {
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

            {
                // println!("updating strategy for gamestate {} . {:?}", self.game_state.current_state_as_string(), utilities);
            } 
            let strategy = self.get_strategy();   
            strategy.update_strategy(utility, &utilities, training_iteration);   
            utility
        }
    }

    fn traverse_chosen_action(&mut self, action: usize, acting_player: Player, current_pot: u8, current_bets: u8, current_checks: u8) -> f64 {
        match action {
            0 => {self.game_state.checkfold();},
            1 => {self.game_state.call_or_bet();},
            2 => {self.game_state.bet();},
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

    fn traverse_flop(&mut self) -> f64 {
        let previous_player = self.game_state.current_player.get();
        let previous_bets = self.game_state.bets_this_round.get();
        let check_before = self.game_state.checks_this_round.get();
        self.game_state.deal_flop();
        let utility = self.traverse_action();
        self.game_state
            .undeal_flop(previous_bets, previous_player, check_before);
        utility
    }

    fn traverse_deal(&mut self) -> f64 {
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