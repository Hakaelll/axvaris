#[cfg(target_os = "linux")]
fn main() {
    if std::env::args_os().len() != 1 {
        eprintln!("Axvaris helper accepts no commands or object paths");
        std::process::exit(2);
    }
    if let Err(error) = sabicheiro_capture::run_linux_helper() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("Axvaris eBPF helper is Linux-only");
    std::process::exit(1);
}
