<div align="center">

# monty
#### "MCTS is cool."

![License](https://img.shields.io/github/license/jw1912/monty?style=for-the-badge)
[![GitHub release (latest by date)](https://img.shields.io/github/v/release/jw1912/monty?style=for-the-badge)](https://github.com/jw1912/akimbo/releases/latest)
[![Commits](https://img.shields.io/github/commits-since/jw1912/monty/latest?style=for-the-badge)](https://github.com/jw1912/akimbo/commits/main)

</div>

## Compiling
Run the following command
```
make <name of game>
```
to compile monty for the given game.

## Supported Games
- Ataxx
- Chess
- Shatranj

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
