use monty::UciLike;

#[cfg(feature = "ataxx")]
use monty::ataxx;

#[cfg(not(feature = "ataxx"))]
use monty::chess;

fn main() {
    let mut args = std::env::args();
    let arg1 = args.nth(1);

    #[cfg(not(feature = "ataxx"))]
    {
        if let Some("bench") = arg1.as_deref() {
            chess::Uci::bench(6);
            return;
        }

        chess::Uci::run();
    }

    #[cfg(feature = "ataxx")]
    {
        if let Some("bench") = arg1.as_deref() {
            ataxx::Uai::bench(5);
            return;
        }

        ataxx::Uai::run();
    }
}
