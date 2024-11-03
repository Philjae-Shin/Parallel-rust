use log::Level;

pub fn init(level: Level, backtrace: bool) {
    let level = std::env::var("RUST_LOG")
        .unwrap_or(level.to_string());
    let backtrace = std::env::var("RUST_BACKTRACE")
        .unwrap_or(if backtrace { "1".to_string() } else { "0".to_string() });
    std::env::set_var("RUST_LOG", &level);
    std::env::set_var("RUST_BACKTRACE", &backtrace);
    let _ = env_logger::try_init();
}

pub fn set_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        hook(panic_info);
        std::process::exit(1);
    }));
}
