use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;

use crate::catalog::{MapDefinition, is_seasonal};

#[derive(Debug, Clone, Copy)]
struct GameMode {
    id: &'static str,
    label: &'static str,
}

const OTHER: GameMode = GameMode {
    id: "other",
    label: "Other",
};

const TAG_MODES: &[(&[&str], GameMode)] = &[
    (
        &["payload_race", "plr"],
        GameMode {
            id: "payload-race",
            label: "Payload Race",
        },
    ),
    (
        &["attack_defend", "attack-defend", "a/d"],
        GameMode {
            id: "attack-defend",
            label: "Attack / Defend",
        },
    ),
    (
        &["king_of_the_hill", "king-of-the-hill", "koth"],
        GameMode {
            id: "king-of-the-hill",
            label: "King of the Hill",
        },
    ),
    (
        &["capture_the_flag", "capture-the-flag", "ctf"],
        GameMode {
            id: "capture-the-flag",
            label: "Capture the Flag",
        },
    ),
    (
        &["player_destruction", "player-destruction", "pd"],
        GameMode {
            id: "player-destruction",
            label: "Player Destruction",
        },
    ),
    (
        &["robot_destruction", "robot-destruction", "rd"],
        GameMode {
            id: "robot-destruction",
            label: "Robot Destruction",
        },
    ),
    (
        &["special_delivery", "special-delivery", "sd"],
        GameMode {
            id: "special-delivery",
            label: "Special Delivery",
        },
    ),
    (
        &["territorial_control", "territorial-control", "tc"],
        GameMode {
            id: "territorial-control",
            label: "Territorial Control",
        },
    ),
    (
        &["passtime", "pass_time", "pass"],
        GameMode {
            id: "passtime",
            label: "PASS Time",
        },
    ),
    (
        &["versus_saxton_hale", "vsh"],
        GameMode {
            id: "versus-saxton-hale",
            label: "Versus Saxton Hale",
        },
    ),
    (
        &["zombie_infection", "zi"],
        GameMode {
            id: "zombie-infection",
            label: "Zombie Infection",
        },
    ),
    (
        &["mann_vs_machine", "mvm"],
        GameMode {
            id: "mann-vs-machine",
            label: "Mann vs. Machine",
        },
    ),
    (
        &["payload", "pl"],
        GameMode {
            id: "payload",
            label: "Payload",
        },
    ),
    (
        &["arena"],
        GameMode {
            id: "arena",
            label: "Arena",
        },
    ),
    (
        &["control_point", "control-point", "cp"],
        GameMode {
            id: "control-point",
            label: "Control Point",
        },
    ),
];

const PREFIX_MODES: &[(&str, GameMode)] = &[
    (
        "plr_",
        GameMode {
            id: "payload-race",
            label: "Payload Race",
        },
    ),
    (
        "pl_",
        GameMode {
            id: "payload",
            label: "Payload",
        },
    ),
    (
        "koth_",
        GameMode {
            id: "king-of-the-hill",
            label: "King of the Hill",
        },
    ),
    (
        "ctf_",
        GameMode {
            id: "capture-the-flag",
            label: "Capture the Flag",
        },
    ),
    (
        "arena_",
        GameMode {
            id: "arena",
            label: "Arena",
        },
    ),
    (
        "pd_",
        GameMode {
            id: "player-destruction",
            label: "Player Destruction",
        },
    ),
    (
        "rd_",
        GameMode {
            id: "robot-destruction",
            label: "Robot Destruction",
        },
    ),
    (
        "sd_",
        GameMode {
            id: "special-delivery",
            label: "Special Delivery",
        },
    ),
    (
        "tc_",
        GameMode {
            id: "territorial-control",
            label: "Territorial Control",
        },
    ),
    (
        "pass_",
        GameMode {
            id: "passtime",
            label: "PASS Time",
        },
    ),
    (
        "vsh_",
        GameMode {
            id: "versus-saxton-hale",
            label: "Versus Saxton Hale",
        },
    ),
    (
        "zi_",
        GameMode {
            id: "zombie-infection",
            label: "Zombie Infection",
        },
    ),
    (
        "mvm_",
        GameMode {
            id: "mann-vs-machine",
            label: "Mann vs. Machine",
        },
    ),
    (
        "cp_",
        GameMode {
            id: "control-point",
            label: "Control Point",
        },
    ),
];

#[derive(Debug, Clone, Serialize)]
pub struct MapStat {
    pub map_index: usize,
    pub map_name: String,
    pub display_name: String,
    pub game_mode_id: String,
    pub game_mode: String,
    pub searching: u32,
    pub relative_percent: f64,
    pub queue_eligible: bool,
    pub seasonal: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModeStat {
    pub game_mode_id: String,
    pub game_mode: String,
    pub map_count: usize,
    pub searching: u64,
    pub share_percent: f64,
    pub relative_percent: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub total_map_selections: u64,
    pub maps_reporting: usize,
    pub eligible_maps: usize,
    pub busiest_map: Option<String>,
    pub busiest_map_searching: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub captured_at: String,
    pub summary: Summary,
    pub maps: Vec<MapStat>,
    pub modes: Vec<ModeStat>,
}

fn game_mode(map: &MapDefinition) -> GameMode {
    for (candidates, mode) in TAG_MODES {
        if candidates
            .iter()
            .any(|candidate| map.tags.iter().any(|tag| tag == candidate))
        {
            return *mode;
        }
    }
    let lower = map.name.to_ascii_lowercase();
    PREFIX_MODES
        .iter()
        .find_map(|(prefix, mode)| lower.starts_with(prefix).then_some(*mode))
        .unwrap_or(OTHER)
}

fn display_name(map_name: &str) -> String {
    let lower = map_name.to_ascii_lowercase();
    let prefixes = [
        "arena_", "koth_", "pass_", "ctf_", "plr_", "mvm_", "vsh_", "cp_", "pd_", "pl_", "rd_",
        "sd_", "tc_", "zi_",
    ];
    let mut value = prefixes
        .iter()
        .find_map(|prefix| lower.starts_with(prefix).then(|| &map_name[prefix.len()..]))
        .unwrap_or(map_name);
    let lower_value = value.to_ascii_lowercase();
    if let Some(stripped) = lower_value.strip_suffix("_event") {
        value = &value[..stripped.len()];
    } else if let Some(position) = lower_value.rfind("_final") {
        let suffix = &lower_value[position + 6..];
        if suffix.chars().all(|character| character.is_ascii_digit()) {
            value = &value[..position];
        }
    }
    let label = value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            if part.len() <= 2 || part.chars().all(|character| character.is_ascii_digit()) {
                part.to_ascii_uppercase()
            } else {
                let mut characters = part.chars();
                match characters.next() {
                    Some(first) => first.to_uppercase().chain(characters).collect(),
                    None => String::new(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    if label.is_empty() {
        map_name.to_owned()
    } else {
        label
    }
}

fn rounded_percent(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let hundredths = numerator
        .saturating_mul(10_000)
        .saturating_add(denominator / 2)
        / denominator;
    f64::from(u32::try_from(hundredths).unwrap_or(u32::MAX)) / 100.0
}

pub fn build_snapshot(
    catalogue: &[MapDefinition],
    counts: &[u32],
    captured_at: DateTime<Utc>,
) -> Snapshot {
    let captured_at = captured_at.to_rfc3339_opts(SecondsFormat::Millis, true);
    let largest = catalogue
        .iter()
        .map(|map| counts.get(map.index).copied().unwrap_or(0))
        .max()
        .unwrap_or(0);
    let mut maps = catalogue
        .iter()
        .filter(|map| map.index > 0 && map.name != "Missing")
        .map(|map| {
            let searching = counts.get(map.index).copied().unwrap_or(0);
            let mode = game_mode(map);
            MapStat {
                map_index: map.index,
                map_name: map.name.clone(),
                display_name: display_name(&map.name),
                game_mode_id: mode.id.to_owned(),
                game_mode: mode.label.to_owned(),
                searching,
                relative_percent: rounded_percent(u64::from(searching), u64::from(largest)),
                queue_eligible: map.queue_eligible,
                seasonal: is_seasonal(map),
                tags: map.tags.clone(),
            }
        })
        .collect::<Vec<_>>();
    maps.sort_by(|left, right| {
        right
            .searching
            .cmp(&left.searching)
            .then_with(|| left.display_name.cmp(&right.display_name))
    });

    let total_map_selections = maps.iter().map(|map| u64::from(map.searching)).sum::<u64>();
    let mut grouped = BTreeMap::<String, ModeStat>::new();
    for map in &maps {
        let group = grouped.entry(map.game_mode_id.clone()).or_insert(ModeStat {
            game_mode_id: map.game_mode_id.clone(),
            game_mode: map.game_mode.clone(),
            map_count: 0,
            searching: 0,
            share_percent: 0.0,
            relative_percent: 0.0,
        });
        group.map_count += 1;
        group.searching += u64::from(map.searching);
    }
    let largest_mode = grouped
        .values()
        .map(|mode| mode.searching)
        .max()
        .unwrap_or(0);
    let mut modes = grouped.into_values().collect::<Vec<_>>();
    for mode in &mut modes {
        mode.share_percent = rounded_percent(mode.searching, total_map_selections);
        mode.relative_percent = rounded_percent(mode.searching, largest_mode);
    }
    modes.sort_by(|left, right| {
        right
            .searching
            .cmp(&left.searching)
            .then_with(|| left.game_mode.cmp(&right.game_mode))
    });
    let busiest = maps.first().filter(|map| map.searching > 0);
    Snapshot {
        captured_at,
        summary: Summary {
            total_map_selections,
            maps_reporting: maps.len(),
            eligible_maps: maps.iter().filter(|map| map.queue_eligible).count(),
            busiest_map: busiest.map(|map| map.display_name.clone()),
            busiest_map_searching: busiest.map_or(0, |map| map.searching),
        },
        maps,
        modes,
    }
}

pub fn write_json<T: Serialize>(value: &T, compact: bool) -> Result<()> {
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    if compact {
        serde_json::to_writer(&mut output, value)?;
    } else {
        serde_json::to_writer_pretty(&mut output, value)?;
    }
    writeln!(output)?;
    Ok(())
}

fn write_csv(
    path: &Path,
    rows: impl FnOnce(&mut csv::Writer<std::fs::File>) -> Result<()>,
) -> Result<()> {
    let file =
        fs::File::create(path).with_context(|| format!("could not create {}", path.display()))?;
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(file);
    rows(&mut writer)?;
    writer.flush()?;
    Ok(())
}

pub struct CsvResult {
    pub directory: PathBuf,
    pub files: [PathBuf; 3],
}

#[allow(clippy::too_many_lines)]
pub fn write_csv_files(
    snapshot: &Snapshot,
    output_root: &Path,
    prefix: &str,
    flat: bool,
) -> Result<CsvResult> {
    let captured = DateTime::parse_from_rfc3339(&snapshot.captured_at)?;
    let stamp = captured.format("%Y%m%dT%H%M%SZ").to_string();
    let directory = if flat {
        output_root.to_path_buf()
    } else {
        output_root.join(&stamp)
    };
    fs::create_dir_all(&directory)?;
    let suffix = if flat {
        format!("-{stamp}")
    } else {
        String::new()
    };
    let files = [
        directory.join(format!("{prefix}-summary{suffix}.csv")),
        directory.join(format!("{prefix}-maps{suffix}.csv")),
        directory.join(format!("{prefix}-modes{suffix}.csv")),
    ];

    write_csv(&files[0], |writer| {
        writer.write_record([
            "captured_at",
            "total_map_selections",
            "maps_reporting",
            "eligible_maps",
            "busiest_map",
            "busiest_map_searching",
        ])?;
        writer.write_record([
            snapshot.captured_at.as_str(),
            &snapshot.summary.total_map_selections.to_string(),
            &snapshot.summary.maps_reporting.to_string(),
            &snapshot.summary.eligible_maps.to_string(),
            snapshot.summary.busiest_map.as_deref().unwrap_or(""),
            &snapshot.summary.busiest_map_searching.to_string(),
        ])?;
        Ok(())
    })?;
    write_csv(&files[1], |writer| {
        writer.write_record([
            "captured_at",
            "map_index",
            "map_name",
            "display_name",
            "game_mode",
            "searching",
            "relative_percent",
            "queue_eligible",
            "seasonal",
            "tags",
        ])?;
        for map in &snapshot.maps {
            writer.write_record([
                snapshot.captured_at.as_str(),
                &map.map_index.to_string(),
                map.map_name.as_str(),
                map.display_name.as_str(),
                map.game_mode.as_str(),
                &map.searching.to_string(),
                &format!("{:.2}", map.relative_percent),
                &map.queue_eligible.to_string(),
                &map.seasonal.to_string(),
                &map.tags.join("|"),
            ])?;
        }
        Ok(())
    })?;
    write_csv(&files[2], |writer| {
        writer.write_record([
            "captured_at",
            "game_mode_id",
            "game_mode",
            "map_count",
            "searching",
            "share_percent",
            "relative_percent",
        ])?;
        for mode in &snapshot.modes {
            writer.write_record([
                snapshot.captured_at.as_str(),
                mode.game_mode_id.as_str(),
                mode.game_mode.as_str(),
                &mode.map_count.to_string(),
                &mode.searching.to_string(),
                &format!("{:.2}", mode.share_percent),
                &format!("{:.2}", mode.relative_percent),
            ])?;
        }
        Ok(())
    })?;
    Ok(CsvResult { directory, files })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Snapshot {
        build_snapshot(
            &[MapDefinition {
                index: 1,
                name: "pl_badwater".to_owned(),
                tags: vec!["payload".to_owned()],
                queue_eligible: true,
            }],
            &[0, 42],
            DateTime::parse_from_rfc3339("2026-07-26T12:00:00Z")
                .unwrap()
                .to_utc(),
        )
    }

    #[test]
    fn builds_pipe_friendly_snapshot() {
        let snapshot = fixture();
        assert_eq!(snapshot.summary.total_map_selections, 42);
        assert_eq!(snapshot.maps[0].display_name, "Badwater");
        assert_eq!(snapshot.modes[0].game_mode_id, "payload");
    }

    #[test]
    fn exports_optional_csv_files() {
        let temp = tempfile::tempdir().unwrap();
        let result = write_csv_files(&fixture(), temp.path(), "tf2-queue", false).unwrap();
        assert!(result.files.iter().all(|file| file.is_file()));
    }

    #[test]
    fn formats_real_map_name_variants() {
        let cases = [
            ("koth_viaduct_event", "Viaduct"),
            ("pl_swiftwater_final1", "Swiftwater"),
            ("cp_a_b", "A B"),
            ("mvm_bigrock", "Bigrock"),
            ("cp_", "cp_"),
        ];
        for (input, expected) in cases {
            assert_eq!(display_name(input), expected, "{input}");
        }
    }

    #[test]
    fn infers_game_mode_from_prefix_without_tags() {
        let map = MapDefinition {
            index: 1,
            name: "ctf_2fort".to_owned(),
            tags: Vec::new(),
            queue_eligible: false,
        };
        assert_eq!(game_mode(&map).id, "capture-the-flag");
    }
}
