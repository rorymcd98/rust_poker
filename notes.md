Fan out
52c9 possible actions == 3 billion
max 4 bets per round means 9 combinations of checkfold, call, bet
max 4 betting roudns
4 * 9 * 52c9 is a lot


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

// What do I mean by opportunity?
Game starts with cards = AKs
Question: "What do you do!?"
Answer: Play this strategy: [C%, F%, C%, B%] <- depends on everything that's happened so far

Ok we BET

Ok now opponent BETs

Question: "What do you do!?"
Answer: Play this strategy: [C%, F%, C%, B%] <- depends on everything that's happened so far

Ok we CALL

CARDS DEALT
A J Q

Question: "What do you do!?" < short for > "What do you do with AKs after BET BET A J Q?"
Answer: Play this strategy: [C%, F%, C%, B%] <- depends on everything that's happened so far

Questions_Answers {
    //"Sonya 123" : {age: ~20, location: prague}
    "What do you do with AKs after BET BET A J Q?" : [10%, 30%, 20%, 40%],
}