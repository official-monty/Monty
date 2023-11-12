const HIDDEN: usize = 768;
const SCALE: i32 = 400;
const QA: i32 = 255;
const QB: i32 = 64;
const QAB: i32 = QA * QB;

#[inline]
fn activate(x: i16) -> i32 {
    i32::from(x.max(0))
}

#[repr(C)]
pub struct ValueNetwork {
    feature_weights: [Accumulator; 768],
    feature_bias: Accumulator,
    output_weights: [Accumulator; 2],
    output_bias: i16,
}

static NNUE: ValueNetwork = unsafe { std::mem::transmute(*include_bytes!("../altair-net.bin")) };

impl ValueNetwork {
    pub fn out(boys: &Accumulator, opps: &Accumulator) -> i32 {
        let mut sum = i32::from(NNUE.output_bias);

        for (&x, &w) in boys.vals.iter().zip(&NNUE.output_weights[0].vals) {
            sum += activate(x) * i32::from(w);
        }

        for (&x, &w) in opps.vals.iter().zip(&NNUE.output_weights[1].vals) {
            sum += activate(x) * i32::from(w);
        }

        sum * SCALE / QAB
    }
}

#[derive(Clone, Copy)]
#[repr(C, align(64))]
pub struct Accumulator {
    vals: [i16; HIDDEN],
}

impl Accumulator {
    pub fn add_feature(&mut self, idx: usize) {
        assert!(idx < 768);
        for (i, d) in self.vals.iter_mut().zip(&NNUE.feature_weights[idx].vals) {
            *i += *d
        }
    }
}

impl Default for Accumulator {
    fn default() -> Self {
        NNUE.feature_bias
    }
}