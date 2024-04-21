#[cfg(feature = "ataxx")]
pub mod ataxx;

#[cfg(not(feature = "ataxx"))]
#[cfg(not(feature = "shatranj"))]
pub mod chess;

#[cfg(feature = "shatranj")]
pub mod shatranj;

use datagen::to_slice_with_lifetime;

use std::{
    fs::File,
    io::{BufRead, BufReader, Write}, time::Instant,
};

const BATCH_SIZE: usize = 16_384;
const BPSB: usize = 1024;

pub fn train<T: TrainablePolicy>(
    threads: usize,
    data_path: String,
    superbatches: usize,
    lr_drop: usize,
) where
    for<'b> &'b T: Send,
{
    let file = File::open(data_path.as_str()).unwrap();
    let mut policy = T::rand_init();
    let positions = file.metadata().unwrap().len() / std::mem::size_of::<T::Data>() as u64;
    let throughput = superbatches * BPSB * BATCH_SIZE;

    println!("# [Info]");
    println!("> Positions {positions}");
    println!("> Epochs {:.2}", throughput as f64 / positions as f64);

    let mut lr = 0.001;
    let mut momentum = T::boxed_and_zeroed();
    let mut velocity = T::boxed_and_zeroed();

    let mut running_error = 0.0;
    let mut sb = 0;
    let mut batch_no = 0;

    'training: loop {
        let cap = 128 * BATCH_SIZE * std::mem::size_of::<T::Data>();
        let file = File::open(data_path.as_str()).unwrap();
        let mut loaded = BufReader::with_capacity(cap, file);

        while let Ok(buf) = loaded.fill_buf() {
            if buf.is_empty() {
                break;
            }

            let data = to_slice_with_lifetime(buf);
            let t = Instant::now();

            for (i, batch) in data.chunks(BATCH_SIZE).enumerate() {
                let mut grad = T::boxed_and_zeroed();
                running_error += gradient_batch::<T>(threads, &policy, &mut grad, batch);
                let adj = 1.0 / batch.len() as f32;
                T::update(&mut policy, &grad, adj, lr, &mut momentum, &mut velocity);

                batch_no += 1;
                print!(
                    "> Superbatch {}/{superbatches} Batch {}/{BPSB} Speed {:.0}\r",
                    sb + 1,
                    batch_no % BPSB,
                    (i * BATCH_SIZE) as f32 / t.elapsed().as_secs_f32()

                );
                let _ = std::io::stdout().flush();

                if batch_no % BPSB == 0 {
                    sb += 1;
                    println!(
                        "> Superbatch {sb}/{superbatches} Running Loss {}",
                        running_error / (BPSB * BATCH_SIZE) as f32
                    );
                    running_error = 0.0;

                    if sb % lr_drop == 0 {
                        lr *= 0.1;
                        println!("Dropping LR to {lr}");
                    }

                    policy.write_to_bin(format!("checkpoints/policy-{sb}.bin").as_str());

                    if sb == superbatches {
                        break 'training;
                    }
                }
            }

            let consumed = buf.len();
            loaded.consume(consumed);
        }
    }
}

fn gradient_batch<T: TrainablePolicy>(
    threads: usize,
    policy: &T,
    grad: &mut T,
    batch: &[T::Data],
) -> f32
where
    for<'b> &'b T: Send,
{
    let size = (batch.len() / threads).max(1);
    let mut errors = vec![0.0; threads];

    std::thread::scope(|s| {
        batch
            .chunks(size)
            .zip(errors.iter_mut())
            .map(|(chunk, err)| {
                s.spawn(move || {
                    let mut inner_grad = T::boxed_and_zeroed();
                    for pos in chunk {
                        T::update_single_grad(pos, policy, &mut inner_grad, err);
                    }
                    inner_grad
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|p| p.join().unwrap())
            .for_each(|part| grad.add_without_explicit_lifetime(&part));
    });

    errors.iter().sum::<f32>()
}

pub trait TrainablePolicy: Send + Sized {
    type Data: Send + Sync;

    fn update(
        policy: &mut Self,
        grad: &Self,
        adj: f32,
        lr: f32,
        momentum: &mut Self,
        velocity: &mut Self,
    );

    fn update_single_grad(pos: &Self::Data, policy: &Self, grad: &mut Self, error: &mut f32);

    fn rand_init() -> Box<Self>;

    fn add_without_explicit_lifetime(&mut self, other: &Self);

    fn boxed_and_zeroed() -> Box<Self> {
        unsafe {
            let layout = std::alloc::Layout::new::<Self>();
            let ptr = std::alloc::alloc_zeroed(layout);
            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            Box::from_raw(ptr.cast())
        }
    }

    fn write_to_bin(&self, path: &str) {
        let size_of = std::mem::size_of::<Self>();

        let mut file = std::fs::File::create(path).unwrap();

        unsafe {
            let ptr: *const Self = self;
            let slice_ptr: *const u8 = std::mem::transmute(ptr);
            let slice = std::slice::from_raw_parts(slice_ptr, size_of);
            file.write_all(slice).unwrap();
        }
    }
}
