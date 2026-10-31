# Output reference

Running `tf2-queue-query` with no subcommand prints one JSON object. The
`summary`, `maps`, `modes`, and `regions` subcommands print just that part.

```json
{
  "captured_at": "2026-10-31T15:18:24.678Z",
  "summary": { ... },
  "maps": [ ... ],
  "modes": [ ... ],
  "regions": { ... }
}
```

`captured_at` is the UTC time of the reply, in RFC 3339 with milliseconds.

## summary

| Field | Meaning |
| --- | --- |
| `total_map_selections` | Sum of every map's `searching`. Not a player count: one player can select several maps. |
| `maps_reporting` | Maps in TF2's catalogue (`master_maps_list` in `items_game.txt`), without the `Missing` placeholder. |
| `eligible_maps` | Of those, maps with matchmaking tags (`queue_eligible`). |
| `busiest_map` | Display name of the map with the most selections, or `null` when nothing is selected. |
| `busiest_map_searching` | That map's selections, or 0. |
| `casual_searching` | Approximate players searching in Casual, summed over data centers. `null` when `regions` is `null`. |
| `casual_searching_is_minimum` | `true` when a Casual data center is at its limit, so `casual_searching` is a minimum. |

## maps

One entry per catalogue map, most selected first, then by display name.

| Field | Meaning |
| --- | --- |
| `map_index` | The map's index in `master_maps_list`; the GC reports counts by this index. |
| `map_name` | File name, for example `pl_badwater`. |
| `display_name` | Readable name, for example `Badwater`. |
| `game_mode_id`, `game_mode` | Mode from the map's matchmaking tags, or its name prefix when it has none. |
| `searching` | Players who currently have this map selected. |
| `relative_percent` | `searching` as a percentage of the most selected map. |
| `queue_eligible` | `true` when the map has matchmaking tags. |
| `seasonal` | `true` for maps tagged `halloween` or `christmas`. |
| `tags` | The map's enabled `rolling_match_tags`, lowercase. |

## modes

Maps grouped by game mode, most selected first.

| Field | Meaning |
| --- | --- |
| `game_mode_id`, `game_mode` | The mode. |
| `map_count` | Catalogue maps in this mode. |
| `searching` | Sum of the mode's map selections. |
| `share_percent` | `searching` as a percentage of `total_map_selections`. |
| `relative_percent` | `searching` as a percentage of the most selected mode. |

## regions

Approximate players searching per data center, by matchmaking type:
`casual`, `mann_up`, `mvm_boot_camp`, and `competitive`. Each list is busiest
first and leaves out data centers with nobody searching. `regions` is `null`
when the reply does not have the expected layout; see
[protocol.md](protocol.md).

| Field | Meaning |
| --- | --- |
| `data_center` | Valve's data center code, for example `fra` or `maa2`. |
| `city`, `country` | City and ISO country code, or `null` for a code this tool does not know yet. |
| `searching` | Approximate players searching, from 1 to 800. |
| `at_limit` | `true` at 800: TF2 does not report beyond that, so the real number is at least 800. |

## CSV files

`tf2-queue-query csv` writes four files with the same fields, each row
starting with `captured_at`:

| File | Rows |
| --- | --- |
| `tf2-queue-summary.csv` | One row with the summary fields. |
| `tf2-queue-maps.csv` | One row per map; `tags` are joined with `\|`. |
| `tf2-queue-modes.csv` | One row per game mode. |
| `tf2-queue-regions.csv` | One row per data center and matchmaking type, with a `matchmaking` column (`casual`, `mann_up`, `mvm_boot_camp`, `competitive`). Only the header when `regions` is `null`. |

The maps CSV has `game_mode` but not `game_mode_id`. Empty cells stand for
`null`.
