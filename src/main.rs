fn main() {
    #[cfg(not(feature = "nonet"))]
    net::run();

    #[cfg(feature = "nonet")]
    nonet::run();
}

#[cfg(not(feature = "nonet"))]
mod net {
    use monty::{ChessState, PolicyNetwork, Uci, ValueNetwork};

    static VALUE: ValueNetwork =
        unsafe { std::mem::transmute(*include_bytes!("../resources/value.network")) };
    static POLICY: PolicyNetwork =
        unsafe { std::mem::transmute(*include_bytes!("../resources/policy.network")) };

    pub fn run() {
        let mut args = std::env::args();
        let arg1 = args.nth(1);

        if let Some("bench") = arg1.as_deref() {
            Uci::bench(ChessState::BENCH_DEPTH, &POLICY, &VALUE);
            return;
        }

        Uci::run(&POLICY, &VALUE);
    }
}

#[cfg(feature = "nonet")]
mod nonet {
    use std::{fs::File, io::Read};

    use monty::{ChessState, Uci};

    unsafe fn read_into_struct_unchecked<T>(path: &str) -> Box<T> {
        let mut f = File::open(path).unwrap();
        let mut x: Box<T> = monty::boxed_and_zeroed();

        let size = std::mem::size_of::<T>();

        unsafe {
            let slice = std::slice::from_raw_parts_mut(x.as_mut() as *mut T as *mut u8, size);
            f.read_exact(slice).unwrap();
        }

        x
    }

    pub fn run() {
        let mut args = std::env::args();
        let arg1 = args.nth(1);

        let policy = unsafe { read_into_struct_unchecked(monty::PolicyFileDefaultName) };

        let value = unsafe { read_into_struct_unchecked(monty::ValueFileDefaultName) };

        if let Some("bench") = arg1.as_deref() {
            Uci::bench(ChessState::BENCH_DEPTH, &policy, &value);
            return;
        }

        Uci::run(&policy, &value);
    }
}
