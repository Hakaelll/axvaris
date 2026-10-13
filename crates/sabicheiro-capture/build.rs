use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=ebpf/network.bpf.c");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux")
        || env::var_os("CARGO_FEATURE_LINUX_EBPF").is_none()
    {
        return;
    }
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"))
        .join("axvaris-network.bpf.o");
    let architecture = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("x86_64") => "x86_64-linux-gnu",
        Ok("aarch64") => "aarch64-linux-gnu",
        other => panic!("unsupported eBPF build architecture: {other:?}"),
    };
    let status = Command::new("clang")
        .args(["-O2", "-g", "-target", "bpf", "-Wall", "-Werror", "-I"])
        .arg(format!("/usr/include/{architecture}"))
        .args(["-c", "ebpf/network.bpf.c", "-o"])
        .arg(&output)
        .status()
        .expect("Linux event capture requires clang and libbpf-dev; use --no-default-features for INET_DIAG only");
    assert!(status.success(), "eBPF compilation failed");
}
