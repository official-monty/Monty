use crate::value::ValueFeatureMap;

const SCALE: i32 = 400;

pub static VALUE: ValueNetwork =
    unsafe { std::mem::transmute(*include_bytes!("../../../resources/chess-value006.bin")) };

#[repr(C)]
pub struct ValueNetwork {
    l1: Layer<768, 256>,
    l2: Layer<256, 16>,
    l3: Layer<16, 16>,
    l4: Layer<16, 16>,
    l5: Layer<16, 16>,
    l6: Layer<16, 1>,
}

impl ValueNetwork {
    pub fn eval<T: ValueFeatureMap>(&self, board: &T) -> i32 {
        let mut l2 = self.l1.biases;

        board.value_feature_map(|feat| {
            for (i, d) in l2.vals.iter_mut().zip(&self.l1.weights[feat].vals) {
                *i += *d;
            }
        });

        let l3 = self.l2.forward(&l2);
        let l4 = self.l3.forward(&l3);
        let l5 = self.l4.forward(&l4);
        let l6 = self.l5.forward(&l5);
        let out = self.l6.forward(&l6);

        (out.vals[0] * SCALE as f32) as i32
    }
}

struct Layer<const M: usize, const N: usize> {
    weights: [Accumulator<N>; M],
    biases: Accumulator<N>,
}

impl<const M: usize, const N: usize> Layer<M, N> {
    fn forward(&self, inputs: &Accumulator<M>) -> Accumulator<N> {
        let mut fwd = self.biases;

        for (i, d) in inputs.vals.iter().zip(self.weights.iter()) {
            let act = screlu(*i);
            for (f, &w) in fwd.vals.iter_mut().zip(d.vals.iter()) {
                *f += w * act;
            }
        }

        fwd
    }
}

#[inline]
fn screlu(x: f32) -> f32 {
    x.clamp(0.0, 1.0).powi(2)
}

#[derive(Clone, Copy)]
#[repr(C)]
struct Accumulator<const HIDDEN: usize> {
    vals: [f32; HIDDEN],
}
