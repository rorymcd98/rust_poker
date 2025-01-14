Fan out
52c9 possible actions == 3 billion
max 4 bets per round means 9 combinations of checkfold, call, bet
max 4 betting roudns
4 * 9 * 52c9 is a lot

// Expected ranges for hand evals
// High Card:              0 - 1277
// One pair:            1278 - 4137
// Two pair:            4138 - 4995
// Three-of-a-kind:     4996 - 5853
// Straight:            5854 - 5863
// Flush:               5864 - 7140
// Full house:          7141 - 7296
// Four of a kind:      7297 - 7452
// Straight flush:      7453 - 7462

Serialisation of the game history
2 bits for action type
6 bits for card
Maximum bit capacity
[6 + 6 = hole cards, 18 = preflop bets, 2 + 6 + 6 + 6 = flop, 18 = flop bets, 2 + 6 = turn, 18 = turn bets, 2 + 6 = river, 18 = river bets] = 120 bits

// Check == "if no bet money - shall we skip?"
// Fold == "I give up"
// Call == "I accept your bet"
// Bet == "I want to bet more money (can only bet £2)"

// Aim of the algorithm - Calculate % of time to take each action, at each oppportunity (a.k.a node).
[Check%, Fold%, Call%, Bet%]

Strategy for 2s Ts vs 5s 6s is [0.018241158, 0.21043459, 0.7713243], with utilities [-0.007873512, -0.016031628, -5.9523315], bets this round 1

Strategy for 5s 6s vs 2s Ts is [0.2731592, 0.38669464, 0.34014618], with utilities [-0.55577767, -6.1325774, 0.5090525], bets this round 1


Main TODOs:

Hand solver
Split the strategy sampling into two phases where we skip actions with marginal actions
Branch pruning (during and after training)
Profiling
Move the hole cards abstraction off of the branch
Reduce the amoutn of cloning
Do a unit test for split pot evaluations




traverser_pot: 54, opponent_pot: 50, current Tra, bets 4, checks 0
Folded Opp
Opp - Action: 1/2
traverser_pot: 54, opponent_pot: 54, current Tra, bets 4, checks 0
Showdown
cards dealt: 5
Utility: 9.843870212623786
Trav seen: 486755280, Trav not seen: 54760860
Opp seen: 474842960, Opp not seen: 64532800
Utility: 9.843870212623786