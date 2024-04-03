pub mod ataxx;
pub mod chess;

use datagen::to_slice_with_lifetime;

use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
};

const BATCH_SIZE: usize = 16_384;

pub fn train<T: TrainablePolicy>(threads: usize, data_path: String, epochs: usize, lr_drop: usize)
where
    for<'b> &'b T: Send,
{
    let file = File::open(data_path.as_str()).unwrap();
    let mut policy = T::rand_init();

    println!("# [Info]");
    println!(
        "> {} Positions",
        file.metadata().unwrap().len() / std::mem::size_of::<T::Data>() as u64,
    );

    let mut lr = 0.001;
    let mut momentum = T::boxed_and_zeroed();
    let mut velocity = T::boxed_and_zeroed();

    for iteration in 1..=epochs {
        println!("# [Training Epoch {iteration}]");
        train_epoch::<T>(
            threads,
            &mut policy,
            lr,
            &mut momentum,
            &mut velocity,
            data_path.as_str(),
        );

        if iteration % lr_drop == 0 {
            lr *= 0.1;
        }

        policy.write_to_bin(format!("checkpoints/ataxx-policy-{iteration}.bin").as_str());
    }
}

fn train_epoch<T: TrainablePolicy>(
    threads: usize,
    policy: &mut T,
    lr: f32,
    momentum: &mut T,
    velocity: &mut T,
    path: &str,
) where
    for<'b> &'b T: Send,
{
    let mut running_error = 0.0;
    let mut num = 0;

    let cap = 128 * BATCH_SIZE * std::mem::size_of::<T::Data>();
    let file = File::open(path).unwrap();
    let size = file.metadata().unwrap().len() as usize / std::mem::size_of::<T::Data>();
    let mut loaded = BufReader::with_capacity(cap, file);
    let mut batch_no = 0;
    let num_batches = (size + BATCH_SIZE - 1) / BATCH_SIZE;

    while let Ok(buf) = loaded.fill_buf() {
        if buf.is_empty() {
            break;
        }

        let data = to_slice_with_lifetime(buf);

        for batch in data.chunks(BATCH_SIZE) {
            let mut grad = T::boxed_and_zeroed();
            running_error += gradient_batch::<T>(threads, policy, &mut grad, batch);
            let adj = 2.0 / batch.len() as f32;
            T::update(policy, &grad, adj, lr, momentum, velocity);

            batch_no += 1;
            print!("> Batch {batch_no}/{num_batches}\r");
            let _ = std::io::stdout().flush();
        }

        num += data.len();
        let consumed = buf.len();
        loaded.consume(consumed);
    }

    println!("> Running Loss: {}", running_error / num as f32);
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
