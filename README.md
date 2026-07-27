# TF2 Queue Query

A small Windows CLI that reads TF2's current matchmaking map-selection stats.
It prints JSON, so you can read it directly, pipe it into `jq`, or use it from
another program.

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

You need Windows x64, TF2 installed, and desktop Steam signed in and online.
Keep TF2 closed while running the query.

## Usage

Download and extract the zip from
[Releases](https://github.com/Rhgx/tf2-queue-query/releases), then run:

```powershell
.\tf2-queue-query.exe
```

That returns a JSON snapshot containing the summary, maps, and game modes.

You can ask for one part:

```powershell
.\tf2-queue-query.exe summary
.\tf2-queue-query.exe maps
.\tf2-queue-query.exe modes
```

A map entry looks like this:

```json
{
  "map_index": 18,
  "map_name": "pl_badwater",
  "display_name": "Badwater",
  "game_mode": "Payload",
  "searching": 159,
  "queue_eligible": true
}
```

The real entry contains a few more fields.

## Pipes

Save the full result:

```powershell
.\tf2-queue-query.exe > snapshot.json
```

Show maps that currently have selections:

```powershell
.\tf2-queue-query.exe maps | jq '.[] | select(.searching > 0) | {map: .display_name, searching}'
```

Get one map:

```powershell
.\tf2-queue-query.exe maps | jq --arg map pl_badwater '.[] | select(.map_name == $map)'
```

Use `--compact` if you want single-line JSON.

## CSV export

CSV is optional:

```powershell
.\tf2-queue-query.exe csv
```

This writes summary, map, and game-mode files under a timestamped `csv-data`
folder. Run `tf2-queue-query.exe csv --help` for output and filename options.

## If something goes wrong

- **TF2 is running:** close it and try again.
- **SteamAPI_Init failed:** make sure desktop Steam is open, online, and signed
  in.
- **TF2 was not found:** pass its folder with `--tf2-root`.
- **GC timeout:** close other TF2 observer tools and retry with `--timeout 60`.
- **SmartScreen warning:** the executable is not code-signed. The release also
  includes a SHA-256 checksum.

Example for a TF2 install in another Steam library:

```powershell
.\tf2-queue-query.exe --tf2-root "D:\SteamLibrary\steamapps\common\Team Fortress 2"
```

## Building

```powershell
cargo test
cargo build --release
.\scripts\package-release.ps1
```

The packaging script puts the zip and checksum in `dist/`.

## License

[MIT](LICENSE)
