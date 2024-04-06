use super::board::Board;

const HIDDEN: usize = 8;
const SCALE: i32 = 400;
const QA: i32 = 255;
const QB: i32 = 64;
const QAB: i32 = QA * QB;

#[repr(C, align(64))]
pub struct ValueNetwork {
    l1_weights: [Accumulator; 768],
    l1_bias: Accumulator,
    l2_weights: Accumulator,
    l2_bias: i16,
}

static NET: ValueNetwork =
    unsafe { std::mem::transmute(*include_bytes!("../../resources/shatranj-value002.bin")) };

impl ValueNetwork {
    pub fn eval(board: &Board) -> i32 {
        let mut acc = Accumulator::default();

        board.features_map(|feat| acc.add(feat));

        let mut eval = 0;

        for (&v, &w) in acc.vals.iter().zip(NET.l2_weights.vals.iter()) {
            eval += screlu(v) * i32::from(w);
        }

        (eval / QA + i32::from(NET.l2_bias)) * SCALE / QAB
    }
}

#[inline]
fn screlu(x: i16) -> i32 {
    i32::from(x).clamp(0, QA).pow(2)
}

#[derive(Clone, Copy)]
#[repr(C)]
struct Accumulator {
    vals: [i16; HIDDEN],
}

impl Default for Accumulator {
    fn default() -> Self {
        NET.l1_bias
    }
}

impl Accumulator {
    fn add(&mut self, idx: usize) {
        for (i, d) in self.vals.iter_mut().zip(&NET.l1_weights[idx].vals) {
            *i += *d;
        }
    }
}
