use sabicheiro_capture::{
    compare_observations, run_linux_event_fixture, run_loopback_truth_suite,
    run_system_provider_fixture, run_windows_event_fixture,
};
use sabicheiro_core::EvidenceObservation;
use std::{env, fs, path::PathBuf, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = env::args().skip(1);
    let mut manifest_path = None;
    let mut observations_path = None;
    let mut report_path = None;
    let mut provider = None;
    let mut poll_interval_ms = 10_u64;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--manifest" => manifest_path = arguments.next().map(PathBuf::from),
            "--observations" => observations_path = arguments.next().map(PathBuf::from),
            "--report" => report_path = arguments.next().map(PathBuf::from),
            "--provider" => provider = arguments.next(),
            "--poll-ms" => {
                poll_interval_ms = arguments
                    .next()
                    .ok_or("--poll-ms requires a value")?
                    .parse()?;
            }
            "--help" | "-h" => {
                println!(
                    "capture-fixture [--provider system] [--poll-ms N] [--manifest FILE] [--observations JOURNAL.json] [--report FILE]"
                );
                return Ok(());
            }
            unknown => return Err(format!("unknown argument: {unknown}").into()),
        }
    }

    if let Some(provider) = provider {
        let interval = Duration::from_millis(poll_interval_ms);
        let run = match provider.as_str() {
            "system" => run_system_provider_fixture(interval),
            "windows-etw" => run_windows_event_fixture(interval),
            "linux-ebpf" => run_linux_event_fixture(interval),
            _ => Err(format!("unsupported fixture provider: {provider}")),
        }
        .map_err(|error| format!("provider fixture failed: {error}"))?;
        let run_json = serde_json::to_string_pretty(&run)?;
        if let Some(path) = report_path {
            fs::write(path, run_json)?;
        } else {
            println!("{run_json}");
        }
        if let Some(path) = manifest_path {
            fs::write(path, serde_json::to_string_pretty(&run.manifest)?)?;
        }
        return Ok(());
    }

    let manifest = run_loopback_truth_suite()?;
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    if let Some(path) = manifest_path {
        fs::write(path, &manifest_json)?;
    } else {
        println!("{manifest_json}");
    }

    if let Some(path) = observations_path {
        let observations: Vec<EvidenceObservation> = serde_json::from_slice(&fs::read(path)?)?;
        let report = compare_observations(&manifest, "external-journal", &observations);
        let report_json = serde_json::to_string_pretty(&report)?;
        if let Some(path) = report_path {
            fs::write(path, report_json)?;
        } else {
            eprintln!("{report_json}");
        }
        if !report.meets_release_targets {
            std::process::exit(2);
        }
    }
    Ok(())
}
