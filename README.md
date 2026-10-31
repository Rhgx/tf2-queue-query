# TF2 Queue Query

A small Windows CLI that reads TF2's current matchmaking map-selection stats
and the approximate number of players searching per data center.
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

That returns a JSON snapshot containing the summary, maps, game modes, and data
centers.

You can ask for one part:

```powershell
.\tf2-queue-query.exe summary
.\tf2-queue-query.exe maps
.\tf2-queue-query.exe modes
.\tf2-queue-query.exe regions
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

The real entry contains a few more fields. Every field is described in
[docs/output.md](https://github.com/Rhgx/tf2-queue-query/blob/main/docs/output.md).

## Data centers

`regions` lists the approximate number of players searching in each data center
for Casual, Mann Up, MvM Boot Camp, and Competitive, busiest first:

```json
{
  "data_center": "fra",
  "city": "Frankfurt",
  "country": "de",
  "searching": 800,
  "at_limit": true
}
```

TF2 stops counting at 800 per data center, so `"at_limit": true` means at least
800, and totals that include such a data center are minimums. These numbers are
this tool's reading of a value TF2 does not explain; see
[docs/protocol.md](https://github.com/Rhgx/tf2-queue-query/blob/main/docs/protocol.md)
for how they are worked out and why there is no "event" queue.

## Pipes

Save the full result:

```powershell
.\tf2-queue-query.exe > snapshot.json
```

Show maps that currently have selections:

```powershell
.\tf2-queue-query.exe maps | jq '.[] | select(.searching > 0) | {map: .display_name, searching}'
```

Show Casual data centers at their limit:

```powershell
.\tf2-queue-query.exe regions | jq '.casual[] | select(.at_limit) | .city'
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

This writes summary, map, game-mode, and data center files under a timestamped
`csv-data` folder. Run `tf2-queue-query.exe csv --help` for output and filename options.

## If something goes wrong

- **TF2 is running:** close it and try again.
- **SteamAPI_Init failed:** make sure desktop Steam is open, online, and signed
  in.
- **TF2 was not found:** pass its folder with `--tf2-root`.
- **GC timeout:** close other TF2 observer tools and retry with `--timeout 60`.
- **`regions` is null:** the reply did not have the expected data center
  layout, so only maps and modes are shown. A warning says why.
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

The packaging script uses the version in `Cargo.toml` and puts the zip and
checksum in `dist/`.

## License

[MIT](LICENSE)
