#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    // The same binary is the desktop app and, with a .langolier, the exported chatbot.
    let argv: Vec<String> = std::env::args().collect();
    if let Some(args) = langolier_lib::server::parse_args(&argv) {
        std::process::exit(langolier_lib::server::main(args));
    }
    #[cfg(feature = "desktop")]
    langolier_lib::run();
    #[cfg(not(feature = "desktop"))]
    {
        eprintln!("Server binary: run it with --serve (see README.md).");
        std::process::exit(2);
    }
}
