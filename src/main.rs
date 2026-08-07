#![deny(unsafe_op_in_unsafe_fn)]
#![warn(clippy::pedantic)]

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
compile_error!("tf2-queue-query currently supports Windows x64 and Linux x86_64 only");

mod catalog;
mod discovery;
mod export;
mod protobuf;
mod regions;
mod steam_gc;

use std::{fs, path::PathBuf, process::ExitCode, time::Duration};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// TF2 installation root. Normally discovered from Steam automatically.
    #[arg(long, global = true)]
    tf2_root: Option<PathBuf>,

    /// Maximum seconds to wait for the TF2 Game Coordinator.
    #[arg(long, default_value_t = 30, global = true, value_parser = clap::value_parser!(u64).range(5..=120))]
    timeout: u64,

    /// Allow running alongside TF2. This can make the GC session unreliable.
    #[arg(long, global = true)]
    allow_tf2_running: bool,

    /// Write single-line JSON instead of pretty JSON.
    #[arg(long, global = true)]
    compact: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print only the per-map statistics as JSON.
    Maps,
    /// Print only the per-game-mode statistics as JSON.
    Modes,
    /// Print only the summary statistics as JSON.
    Summary,
    /// Print only the approximate number searching per data center as JSON.
    Regions,
    /// Write summary, map, mode, and data center CSV files instead of JSON.
    Csv(CsvOptions),
}

#[derive(Debug, clap::Args)]
struct CsvOptions {
    /// Root output directory. A timestamped child folder is created by default.
    #[arg(long, default_value = "csv-data")]
    out: PathBuf,

    /// Prefix used for all CSV filenames.
    #[arg(long, default_value = "tf2-queue", value_parser = valid_prefix)]
    prefix: String,

    /// Put timestamped filenames directly in --out instead of a timestamp folder.
    #[arg(long)]
    flat: bool,
}

fn valid_prefix(value: &str) -> std::result::Result<String, String> {
    if !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        Ok(value.to_owned())
    } else {
        Err("prefix may contain only letters, numbers, underscores, and hyphens".to_owned())
    }
}

fn capture(cli: &Cli) -> Result<export::Snapshot> {
    if steam_gc::tf2_is_running() && !cli.allow_tf2_running {
        bail!("TF2 is running; close it first, or explicitly pass --allow-tf2-running");
    }
    eprintln!("Locating the installed Team Fortress 2 client...");
    let tf2_root = discovery::find_tf2_root(cli.tf2_root.clone())?;
    let items_game_path = tf2_root
        .join("tf")
        .join("scripts")
        .join("items")
        .join("items_game.txt");
    eprintln!(
        "Reading map catalogue from {}...",
        items_game_path.display()
    );
    let items_game = fs::read_to_string(&items_game_path)
        .with_context(|| format!("could not read {}", items_game_path.display()))?;
    let catalogue = catalog::parse_master_maps_list(&items_game)?;
    eprintln!("Requesting one passive matchmaking-stat snapshot from Steam GC...");
    let body = steam_gc::request_stats(&tf2_root, Duration::from_secs(cli.timeout))?;
    let counts = protobuf::decode_map_counts(&body)?;
    // Data center numbers are extra; if they cannot be read, the map counts are still returned.
    let regions = match protobuf::decode_region_groups(&body) {
        Ok(groups) => {
            let regions = regions::build_regions(&groups);
            if regions.is_none() {
                eprintln!(
                    "warning: data center numbers were not in the expected layout; regions is null"
                );
            }
            regions
        }
        Err(error) => {
            eprintln!("warning: could not read data center numbers: {error:#}; regions is null");
            None
        }
    };
    Ok(export::build_snapshot(
        &catalogue,
        &counts,
        regions,
        Utc::now(),
    ))
}

fn validate_cli(cli: &Cli) -> Result<()> {
    if cli.compact && matches!(cli.command, Some(Command::Csv(_))) {
        bail!("--compact applies only to JSON output and cannot be used with csv");
    }
    Ok(())
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    validate_cli(&cli)?;
    let snapshot = capture(&cli)?;
    match &cli.command {
        None => export::write_json(&snapshot, cli.compact),
        Some(Command::Maps) => export::write_json(&snapshot.maps, cli.compact),
        Some(Command::Modes) => export::write_json(&snapshot.modes, cli.compact),
        Some(Command::Summary) => export::write_json(&snapshot.summary, cli.compact),
        Some(Command::Regions) => match &snapshot.regions {
            Some(regions) => export::write_json(regions, cli.compact),
            None => bail!("the TF2 GC reply did not include data center numbers"),
        },
        Some(Command::Csv(options)) => {
            let result =
                export::write_csv_files(&snapshot, &options.out, &options.prefix, options.flat)?;
            for file in &result.files {
                println!("{}", file.display());
            }
            eprintln!(
                "Wrote {} maps, {} modes and {} data center rows to {}.",
                snapshot.maps.len(),
                snapshot.modes.len(),
                snapshot.regions.as_ref().map_or(0, |regions| {
                    regions.groups().iter().map(|(_, stats)| stats.len()).sum()
                }),
                result.directory.display()
            );
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn clap_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn compact_rejects_csv_output() {
        let cli = Cli::try_parse_from(["tf2-queue-query", "--compact", "csv"]).unwrap();
        assert!(validate_cli(&cli).is_err());
    }
}
