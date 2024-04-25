use monty::{shatranj::Uci, UciLike, ValueNetwork};

static VALUE: ValueNetwork<768, 8> =
    unsafe { std::mem::transmute(*include_bytes!(concat!("../../", env!("EVALFILE")))) };

static POLICY: () = ();

fn main() {
    let mut args = std::env::args();
    let arg1 = args.nth(1);

    if let Some("bench") = arg1.as_deref() {
        Uci::bench(6, &POLICY, &VALUE);
        return;
    }

    Uci::run(&POLICY, &VALUE);
}