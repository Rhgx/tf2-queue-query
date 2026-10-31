# Changelog

## 0.2.0 - 2026-10-31

- **Players searching per data center.** See roughly how many players are
  searching in each data center for Casual, Mann Up, MvM Boot Camp, and
  Competitive. It's in the new `regions` command, in the JSON and CSV output,
  and as `casual_searching` in the summary.
- **Linux support (experimental).** A Linux x86_64 build, based on
  [Kino-Cineaste's fork](https://github.com/Kino-Cineaste/tf2-queue-query). It
  needs the native Steam client and hasn't been confirmed against a live Steam
  session yet.

## 0.1.0 - 2026-07-26

- First Windows x64 release.
- Queries TF2's passive map-selection statistics.
- Prints the full snapshot or individual map, mode, and summary data as JSON.
- Can optionally export summary, map, and mode CSV files.
- Finds TF2 across Steam library folders.
- Includes a local zip packager and GitHub release workflow.
