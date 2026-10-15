#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--selftest") {
        let report = args
            .next()
            .unwrap_or_else(|| "pegatina-selftest.txt".into());
        std::process::exit(pegatina_app::selftest::run(&report));
    }
    pegatina_app::run();
}
