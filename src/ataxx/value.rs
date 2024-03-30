use super::board::Board;

const HIDDEN: usize = 128;
const SCALE: i32 = 400;
const QA: i32 = 255;
const QB: i32 = 64;
const QAB: i32 = QA * QB;
const PER_TUPLE: usize = 3usize.pow(4);

#[repr(C)]
pub struct ValueNetwork {
    l1_weights: [Accumulator; 2916],
    l1_bias: Accumulator,
    l2_weights: Accumulator,
    l2_bias: i16,
}

static NET: ValueNetwork =
    unsafe { std::mem::transmute(*include_bytes!("../../resources/ataxx-value011.bin")) };

impl ValueNetwork {
    pub fn eval(board: &Board) -> i32 {
        let mut acc = Accumulator::default();

        let boys = board.boys();
        let opps = board.opps();

        for i in 0..6 {
            for j in 0..6 {
                const POWERS: [usize; 4] = [1, 3, 9, 27];
                const MASK: u64 = 0b0001_1000_0011;

                let tuple = 6 * i + j;
                let mut feat = PER_TUPLE * tuple;

                let offset = 7 * i + j;
                let mut b = (boys >> offset) & MASK;
                let mut o = (opps >> offset) & MASK;

                while b > 0 {
                    let mut sq = b.trailing_zeros() as usize;
                    if sq > 6 {
                        sq -= 5;
                    }

                    feat += POWERS[sq];

                    b &= b - 1;
                }

                while o > 0 {
                    let mut sq = o.trailing_zeros() as usize;
                    if sq > 6 {
                        sq -= 5;
                    }

                    feat += 2 * POWERS[sq];

                    o &= o - 1;
                }

                acc.add(feat);
            }
        }

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
#[repr(C, align(64))]
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
