# TF2 Queue Query

A small command-line tool for Windows and Linux that reads TF2's current
matchmaking map-selection stats and the approximate number of players
searching per data center. It prints JSON, so you can read it directly, pipe
it into `jq`, or use it from another program.

It uses your existing desktop Steam session. It never asks for your password,
Steam Guard code, token, or Web API key.

> [!IMPORTANT]
> The map numbers are **selections, not unique players**. One player can select
> several maps, so adding every map together does not give a worldwide player
> count.

## Before using it

This is an unofficial experiment and Valve does not support it. It uses
undocumented TF2 Game Coordinator messages which could stop working at any
time. It does not inject into TF2 or queue for a match, but I still cannot
promise anything about account safety. Use it at your own risk.

You need Windows x64 or Linux x86_64, TF2 installed, and desktop Steam signed
in and online. Keep TF2 closed while running the query.

## Quick start

Download and extract the archive for your platform from
[Releases](https://github.com/Rhgx/tf2-queue-query/releases), then run it from
that folder (`.\tf2-queue-query.exe` in PowerShell, `./tf2-queue-query` on
Linux):

```text
tf2-queue-query            full JSON snapshot
tf2-queue-query maps       selections per map
tf2-queue-query modes      selections per game mode
tf2-queue-query regions    approximate players searching per data center
tf2-queue-query csv        the same data as CSV files
```

## Linux

Linux support is experimental. It is based on
[Kino-Cineaste's fork](https://github.com/Kino-Cineaste/tf2-queue-query) and is
built and tested in CI, but has not yet been confirmed against a live Steam
session here. It needs the native Steam client; Flatpak and Snap installs are
not supported. Please open an issue with the output if it works or fails for
you.

## Documentation

- [Usage](docs/usage.md): commands, options, `jq` examples, CSV export, and
  troubleshooting.
- [Output](docs/output.md): every JSON field and CSV file.
- [How it works](docs/protocol.md): the Game Coordinator messages, what the data
  center numbers mean, and the unused event match group.
- [Building](docs/building.md): building from source and cutting a release.

## License

[MIT](LICENSE)
