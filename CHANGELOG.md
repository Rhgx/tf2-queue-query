# Changelog

## Unreleased

- Adds the approximate number of players searching per data center for Casual,
  Mann Up, MvM Boot Camp, and Competitive, from the same stats reply. New
  `regions` section and subcommand; data centers at TF2's limit of 800 are
  marked `at_limit`.
- Adds `casual_searching` and `casual_searching_is_minimum` to the summary.
- CSV export writes a fourth file, `tf2-queue-regions.csv`, and two new summary
  columns at the end.
- If the data center part of a reply cannot be read, `regions` is `null` with
  a warning; maps and modes still work.
- New docs: `docs/output.md` (every field) and `docs/protocol.md` (how the
  query works, including the unused event match group).
- The packaging script now takes its version from `Cargo.toml` by default.

## 0.1.0 - 2026-07-26

- First Windows x64 release.
- Queries TF2's passive map-selection statistics.
- Prints the full snapshot or individual map, mode, and summary data as JSON.
- Can optionally export summary, map, and mode CSV files.
- Finds TF2 across Steam library folders.
- Includes a local zip packager and GitHub release workflow.
