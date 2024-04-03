use monty::{TunableParams, UciLike};

#[cfg(feature = "ataxx")]
use monty::ataxx;

#[cfg(not(feature = "ataxx"))]
use monty::chess;

fn main() {
    let mut args = std::env::args();
    let arg1 = args.nth(1);

    let params = TunableParams::default();

    #[cfg(not(feature = "ataxx"))]
    {
        if let Some("bench") = arg1.as_deref() {
            chess::Uci::bench(5, &params);
            return;
        }

        chess::Uci::run();
    }

    #[cfg(feature = "ataxx")]
    {
        if let Some("bench") = arg1.as_deref() {
            ataxx::Uai::bench(5, &params);
            return;
        }

        ataxx::Uai::run();
    }
}
