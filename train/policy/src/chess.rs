use datagen::{impls::chess::ChessPolicyData, Rand};
use goober::{FeedForwardNetwork, OutputLayer, SparseVector};
use monty::chess::{Move, PolicyNetwork, SubNet};

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
        for (i, subnet) in policy.subnets.iter_mut().enumerate() {
            subnet.adam(
                &grad.subnets[i],
                &mut momentum.subnets[i],
                &mut velocity.subnets[i],
                adj,
                lr,
            );
        }
    }

    fn update_single_grad(pos: &Self::Data, policy: &Self, grad: &mut Self, error: &mut f32) {
        let board = pos.board;

        let mut feats = SparseVector::with_capacity(32);
        board.map_policy_features(|feat| feats.push(feat));

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

            let from_out = policy.subnets[from].out_with_layers(&feats);
            let to_out = policy.subnets[to].out_with_layers(&feats);

            let score = from_out.output_layer().dot(&to_out.output_layer());

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

            policy.subnets[from].backprop(
                &feats,
                &mut grad.subnets[from],
                factor * to_out.output_layer(),
                &from_out,
            );

            policy.subnets[to].backprop(
                &feats,
                &mut grad.subnets[to],
                factor * from_out.output_layer(),
                &to_out,
            );
        }
    }

    fn rand_init() -> Box<Self> {
        let mut policy = Self::boxed_and_zeroed();

        let mut rng = Rand::with_seed();
        for subnet in policy.subnets.iter_mut() {
            *subnet = SubNet::from_fn(|| rng.rand_f32(0.2));
        }

        policy
    }

    fn add_without_explicit_lifetime(&mut self, rhs: &Self) {
        for (i, j) in self.subnets.iter_mut().zip(rhs.subnets.iter()) {
            *i += j;
        }
    }
}
