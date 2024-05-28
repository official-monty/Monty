<div align="center">

# monty
#### "MCTS is cool."

![License](https://img.shields.io/github/license/jw1912/monty?style=for-the-badge)
[![GitHub release (latest by date)](https://img.shields.io/github/v/release/jw1912/monty?style=for-the-badge)](https://github.com/jw1912/monty/releases/latest)
[![Commits](https://img.shields.io/github/commits-since/jw1912/monty/latest?style=for-the-badge)](https://github.com/jw1912/monty/commits/main)

</div>

## Compiling
You need a network file, and run the following command
```
make <name of game (lowercase)> EXE=<output path> EVALFILE=<path to network file>
```
to compile monty for the given game (chess, ataxx, shatranj).

## Originality Status

The first version (0.1.0) used external data for value networks and self-generated policy data. The networks were then reset
completely, and all future versions are trained exclusively on monty's own data, generated from scratch with uniform policy
and material counting value.

## Credits
Thanks to everyone at SWE as usual, in particular Cosmo (Viridithas) and Zuppa (Alexandria), for helping with data generation, and Plutie, for running an LTC tune.

## ELO

<div align="center">

| Version | Release Date | CCRL 40/15 | CCRL Blitz | CCRL FRC | Notes |
| :-: | :-: | :-: | :-: | :-: | :-: |
| [1.0.0](https://github.com/jw1912/monty/releases/tag/v1.0.0) | 28th May 2024 | TBD | TBD | TBD | Fully Original Data |
| [0.1.0](https://github.com/jw1912/monty/releases/tag/v0.1.0) | 26th March 2024 | - | - | 2974 | First Release |

</div>

## How it works

Monte-Carlo Tree Search is broken down into 4 steps to build a tree.

To begin with, only the root node is in the tree.

1. **Selection** of a node in the tree which has at least one unexplored child.
2. **Expansion** to one of the unexplored children, and adding it to the tree.
3. **Simulation** of the result of the game.
4. **Backpropogation** of the result to the root.

Unfortunately, MCTS in its purest form (random selection and random simulation to the end of the game)
is really bad.

Instead, **selection** is done via PUCT, a combination of a **policy network** which indicates the quality of the child nodes,
and the PUCT formula to control exploration vs exploitation of these child nodes.

And **simulation** is replaced with a neural network evaluation, called the **value network**.
