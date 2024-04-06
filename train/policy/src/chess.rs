use datagen::{impls::chess::ChessPolicyData, Rand};
use goober::{FeedForwardNetwork, OutputLayer};
use monty::chess::{consts::Flag, Move, PolicyNetwork, SubNet};

use crate::TrainablePolicy;

impl TrainablePolicy for PolicyNetwork {
    type Data = ChessPolicyData;

    fn update(
        policy: &mut Self,
        grad: &Self,
        adj: f32,
        lr: f32,
        momentum: &mut Self,
        velocity: &mut Self,
    ) {
        for (i, subnet) in policy.weights.iter_mut().enumerate() {
            subnet.adam(
                &grad.weights[i],
                &mut momentum.weights[i],
                &mut velocity.weights[i],
                adj,
                lr,
            );
        }

        for (i, p) in policy.hce.iter_mut().enumerate() {
            const B1: f32 = 0.9;
            const B2: f32 = 0.999;

            let g = adj * grad.hce[i];
            let m = &mut momentum.hce[i];
            let v = &mut velocity.hce[i];

            *m = B1 * *m + (1. - B1) * g;
            *v = B2 * *v + (1. - B2) * g * g;
            *p -= lr * *m / (v.sqrt() + 0.000_000_01);
        }
    }

    fn update_single_grad(pos: &Self::Data, policy: &Self, grad: &mut Self, error: &mut f32) {
        let board = pos.board;

        let feats = board.get_features();

        let mut policies = Vec::with_capacity(pos.num);
        let mut total = 0.0;
        let mut total_visits = 0;
        let mut max = -1000.0;

        let flip = board.flip_val();

        for training_mov in &pos.moves[..pos.num] {
            let mov = <Move as From<u16>>::from(training_mov.mov);

            let visits = training_mov.visits;
            let from = usize::from(mov.from() ^ flip);
            let to = 64 + usize::from(mov.to() ^ flip);

            let from_out = policy.weights[from].out_with_layers(&feats);
            let to_out = policy.weights[to].out_with_layers(&feats);

            let net_out = from_out.output_layer().dot(&to_out.output_layer());

            let score = net_out + policy.hce(&mov, &board);

            if score > max {
                max = score;
            }

            total_visits += visits;
            policies.push((mov, visits, score, from_out, to_out));
        }

        for (_, _, score, _, _) in policies.iter_mut() {
            *score = (*score - max).exp();
            total += *score;
        }

        for (mov, visits, score, from_out, to_out) in policies {
            let from = usize::from(mov.from() ^ flip);
            let to = 64 + usize::from(mov.to() ^ flip);

            let ratio = score / total;

            let expected = visits as f32 / total_visits as f32;
            let err = ratio - expected;

            *error -= expected * ratio.ln();

            let factor = err;

            policy.weights[from].backprop(
                &feats,
                &mut grad.weights[from],
                factor * to_out.output_layer(),
                &from_out,
            );

            policy.weights[to].backprop(
                &feats,
                &mut grad.weights[to],
                factor * from_out.output_layer(),
                &to_out,
            );

            if board.see(&mov, -108) {
                grad.hce[0] += factor;
            }

            if [Flag::QPR, Flag::QPC].contains(&mov.flag()) {
                grad.hce[1] += factor;
            }

            if mov.is_capture() {
                grad.hce[2] += factor;

                let diff =
                    board.get_pc(1 << mov.to()) as i32 - board.get_pc(1 << mov.from()) as i32;
                grad.hce[3] += factor * diff as f32;
            }
        }
    }

    fn rand_init() -> Box<Self> {
        let mut policy = Self::boxed_and_zeroed();

        let mut rng = Rand::with_seed();
        for subnet in policy.weights.iter_mut() {
            *subnet = SubNet::from_fn(|| rng.rand_f32(0.2));
        }

        policy
    }

    fn add_without_explicit_lifetime(&mut self, rhs: &Self) {
        for (i, j) in self.weights.iter_mut().zip(rhs.weights.iter()) {
            *i += j;
        }

        for (i, j) in self.hce.iter_mut().zip(rhs.hce.iter()) {
            *i += *j;
        }
    }
}
