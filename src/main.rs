use monty::UciLike;

fn main() {
    let mut args = std::env::args();
    let arg1 = args.nth(1);

    #[cfg(not(feature = "ataxx"))]
    {
        #[cfg(not(feature = "shatranj"))]
        {
            if let Some("bench") = arg1.as_deref() {
                monty::chess::Uci::bench(4);
                return;
            }

            monty::chess::Uci::run();
        }

        #[cfg(feature = "shatranj")]
        {
            if let Some("bench") = arg1.as_deref() {
                monty::shatranj::Uci::bench(6);
                return;
            }

            monty::shatranj::Uci::run();
        }
    }

    #[cfg(feature = "ataxx")]
    {
        if let Some("bench") = arg1.as_deref() {
            monty::ataxx::Uai::bench(5);
            return;
        }

        monty::ataxx::Uai::run();
    }
}
