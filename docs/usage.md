# Usage

The examples use `tf2-queue-query`. Run it as `.\tf2-queue-query.exe` in
PowerShell on Windows and as `./tf2-queue-query` on Linux, from the folder you
extracted it to.

## Commands

| Command | Prints |
| --- | --- |
| `tf2-queue-query` | The full JSON snapshot: summary, maps, modes, and regions. |
| `tf2-queue-query summary` | Only the summary. |
| `tf2-queue-query maps` | Only the per-map selections. |
| `tf2-queue-query modes` | Only the per-game-mode selections. |
| `tf2-queue-query regions` | Only the approximate players searching per data center. |
| `tf2-queue-query csv` | Writes CSV files instead of JSON. |

Every field is described in [output.md](output.md).

## Options

| Option | Meaning |
| --- | --- |
| `--tf2-root <folder>` | TF2's install folder, when it is not found automatically. |
| `--timeout <seconds>` | How long to wait for the Game Coordinator, 5 to 120. Default 30. |
| `--compact` | Single-line JSON. Not available with `csv`. |
| `--allow-tf2-running` | Run even though TF2 is open. The GC session can be unreliable then. |

For example, with TF2 in another Steam library:

```powershell
.\tf2-queue-query.exe --tf2-root "D:\SteamLibrary\steamapps\common\Team Fortress 2"
```

```bash
./tf2-queue-query --tf2-root "/mnt/games/SteamLibrary/steamapps/common/Team Fortress 2"
```

## jq examples

These work the same in PowerShell and bash.

Save the full result:

```text
tf2-queue-query > snapshot.json
```

Show maps that currently have selections:

```text
tf2-queue-query maps | jq '.[] | select(.searching > 0) | {map: .display_name, searching}'
```

Get one map:

```text
tf2-queue-query maps | jq --arg map pl_badwater '.[] | select(.map_name == $map)'
```

Show Casual data centers at their limit:

```text
tf2-queue-query regions | jq '.casual[] | select(.at_limit) | .city'
```

## CSV export

```text
tf2-queue-query csv
```

This writes summary, map, game-mode, and data center files to a timestamped
folder under `csv-data`. Options:

| Option | Meaning |
| --- | --- |
| `--out <folder>` | Root folder. Default `csv-data`. |
| `--prefix <name>` | File name prefix. Default `tf2-queue`. Letters, numbers, `-` and `_` only. |
| `--flat` | Put timestamped files directly in `--out` instead of a timestamp folder. |

## If something goes wrong

- **TF2 is running:** close it and try again.
- **SteamAPI_Init failed:** make sure desktop Steam is open, online, and signed
  in. On Linux it has to be the native Steam client, not Flatpak or Snap.
- **TF2 was not found:** pass its folder with `--tf2-root`.
- **GC timeout:** close other TF2 observer tools and retry with `--timeout 60`.
- **`regions` is null:** the reply did not have the expected data center
  layout, so only maps and modes are shown. A warning says why.
- **SmartScreen warning on Windows:** the executable is not code-signed. Each
  release archive has a SHA-256 checksum file next to it.
