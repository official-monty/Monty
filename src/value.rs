const SCALE: i32 = 400;
const QA: i32 = 255;
const QB: i32 = 64;
const QAB: i32 = QA * QB;

#[repr(C, align(64))]
pub struct ValueNetwork<const INPUT: usize, const HIDDEN: usize> {
    l1_weights: [Accumulator<HIDDEN>; INPUT],
    l1_bias: Accumulator<HIDDEN>,
    l2_weights: Accumulator<HIDDEN>,
    l2_bias: i16,
}

pub trait ValueFeatureMap {
    fn value_feature_map<F: FnMut(usize)>(&self, f: F);
}

#[derive(Clone, Copy)]
#[repr(C)]
struct Accumulator<const HIDDEN: usize> {
    vals: [i16; HIDDEN],
}

#[inline]
fn screlu(x: i16) -> i32 {
    i32::from(x).clamp(0, QA).pow(2)
}

impl<const INPUT: usize, const HIDDEN: usize> ValueNetwork<INPUT, HIDDEN> {
    pub fn eval<T: ValueFeatureMap>(&self, board: &T) -> i32 {
        let mut acc = self.l1_bias;

        board.value_feature_map(|feat| {
            for (i, d) in acc.vals.iter_mut().zip(&self.l1_weights[feat].vals) {
                *i += *d;
            }
        });

        let mut eval = 0;

        for (&v, &w) in acc.vals.iter().zip(self.l2_weights.vals.iter()) {
            eval += screlu(v) * i32::from(w);
        }

        (eval / QA + i32::from(self.l2_bias)) * SCALE / QAB
    }
}
