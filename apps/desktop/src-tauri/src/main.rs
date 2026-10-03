#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let _ = std::fs::write("looma_panic.log", format!("PANIC: {info}\n"));
    }));

    looma_desktop::run();
}
