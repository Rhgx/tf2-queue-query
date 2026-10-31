# Changelog

## 0.2.0 - 2026-10-31

- Experimental Linux x86_64 support, based on Kino-Cineaste's fork.
- Approximate players searching per data center: `regions` in the JSON and
  CSV, a `regions` subcommand, and `casual_searching` in the summary.
- Documentation moved from the README into `docs/`.
- Releases include a Linux `.tar.gz`, built by the release workflow, which
  replaces the PowerShell packaging script.

## 0.1.0 - 2026-07-26

- First Windows x64 release.
- Queries TF2's passive map-selection statistics.
- Prints the full snapshot or individual map, mode, and summary data as JSON.
- Can optionally export summary, map, and mode CSV files.
- Finds TF2 across Steam library folders.
- Includes a local zip packager and GitHub release workflow.
