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
cargo r -r --bin monty-engine
```
and the executable will be located at `target/release/monty-engine[.exe]`.

## How it works

Monte-Carlo Tree Search is broken down into 4 steps to build a tree.

To begin with, only the root node is in the tree.

1. **Selection** of a node in the tree which has at least one unexplored child.
2. **Expansion** to one of the unexplored children, and adding it to the tree.
3. **Simulation** of the result of the game.
4. **Backpropogation** of the result to the root.

Unfortunately, MCTS in its purest form (random selection and random simulation to the end of the game)
is really bad.

Instead **selection** is replaced with PUCT, a combination of a **policy network** which indicates the quality of the child nodes,
and the PUCT formula to control exploration vs exploitation of these child nodes.

And **simulation** is replaced with quiescence search of the node, backed by a neural network evaluation, called the **value network**.

The value network is currently trained using my NNUE trainer, [bullet](https://github.com/jw1912/bullet), and the policy network is trained using the
trainer in this repo, [monty-train](/monty-train).

## Policy Network Architecture
A sort of pseudo-rust code for the net architecture:
```rust
/// Takes in the features of the position
/// and outputs an N-dimensional vector
pub struct SquareSubNet { ... }

/// Hand-crafted features based on the move
/// and position, e.g. a bonus for moves that
/// are captures. Returns a scalar.
pub struct HandCraftedSubNet { ... }

/// Takes in a move and position and outputs a raw policy value,
/// which needs to be softmaxed with the values for all the other
/// moves in the position in order to be used.
pub struct PolicyNetwork {
    from_subnets: [SquareSubNet; 64],
    to_subnets: [SquareSubNet; 64],
    hand_crafted_subnet: HandCraftedSubNet
}

impl PolicyNetwork {
    pub fn evaluate(&self, pos: &Position, mov: Move) -> f32 {
        let from_vec = self.from_subnets[mov.from()].evaluate(pos);
        let to_vec = self.to_subnets[mov.to()].evaluate(pos);

        let hce = self.hand_crafted_subnet.evaluate(pos, mov);

        hce + from_vec.dot(to_vec)
    }
}
```

## Value Network Architecture
A `(768 -> 1024)x2 -> 1` network trained with my NNUE trainer, [bullet](https://github.com/jw1912/bullet).
