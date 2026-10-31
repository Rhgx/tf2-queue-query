# How it works

The tool sends TF2's Game Coordinator (GC) one passive matchmaking stats
request and prints the reply. It does not start TF2, join a queue, or change
anything on the account.

Message names below come from TF2's protobuf definitions as published in
[SteamDatabase/Protobufs](https://github.com/SteamDatabase/Protobufs/blob/master/tf2/tf_gcmessages.proto).
Valve does not document what the values mean, so the interpretations here are
this tool's.

## Session

1. Load TF2's own Steam API library as app 440 (`bin/x64/steam_api64.dll` on
   Windows, `bin/linux64/libsteam_api.so` on Linux) and attach to the
   signed-in desktop Steam session.
2. Send `k_EMsgGCClientHello` (4006) every 2.5 seconds until the GC answers
   with `k_EMsgGCClientWelcome` (4004).
3. Send `k_EMsgGC_TFClientInit` (6536), `CMsgTFClientInit` with
   `client_version` from `tf/steam.inf` and `language` 0.
4. Send `k_EMsgGCRequestMatchMakerStats` (6524), an empty
   `CMsgGCRequestMatchMakerStats`.
5. Read `k_EMsgGCMatchMakerStatsResponse` (6525) and exit.

One run sends one stats request.

## The reply

```proto
message CMsgGCMatchMakerStatsResponse {
  repeated uint32 map_count = 1;
  repeated CMsgGCMatchGroupDataCenterPopulation matchgroup_data_center_population = 2;
}

message CMsgGCMatchGroupDataCenterPopulation {
  repeated CMsgGCDataCenterPopulation data_center_population = 1;
}

message CMsgGCDataCenterPopulation {
  optional string name = 1;
  optional float health_ratio = 2;
}
```

### Map selections

`map_count` holds one number per map, in the order of `master_maps_list` in
`tf/scripts/items/items_game.txt`, which is why the tool reads that file. Each
number is how many players currently have that map selected. Players can
select many maps, so the numbers add up to more than the number of players.

### Data centers

`matchgroup_data_center_population` holds one list per TF2 match group, in the
order of the `ETFMatchGroup` enum:

| Index | Match group | In the output |
| --- | --- | --- |
| 0 | `MvM_Practice` | `mvm_boot_camp` |
| 1 | `MvM_MannUp` | `mann_up` |
| 2 | `Ladder_6v6` | `competitive` |
| 3, 4 | `Ladder_9v9`, `Ladder_12v12` | Left out: not offered, always empty. |
| 5, 6 | `Casual_6v6`, `Casual_9v9` | Left out: not offered, always empty. |
| 7 | `Casual_12v12` | `casual` |
| 8 | `Event_Placeholder` | Left out: see below. |

Each list entry is a data center code (`name`, for example `fra` or `maa2`) and
a `health_ratio`. TF2 does not say what the ratio measures, but it behaves like
a count of players searching divided by 800:

- every value is a whole multiple of 1/800 (all 1,423 values in 20 sampled
  replies),
- it never goes above 1.0, and the busiest Casual data centers sit at exactly
  1.0,
- read as players, Casual's total is about one sixth of the map selections in
  every sample, which fits players selecting several maps each.

So the tool reports `round(health_ratio * 800)` as the approximate number
searching, and a ratio of 1.0 as `at_limit`: at least 800, real number unknown.
Data centers with nobody searching are left out.

### The event group

`Event_Placeholder` is a match group slot TF2 reserves for a separate,
event-only matchmaking queue. It is not used today. Seasonal events such as
Scream Fortress add their maps to Casual instead, so those players are counted
under `casual`. The event group was empty in every reply checked, including
replies taken during Scream Fortress, so the tool leaves it out. If Valve ever
starts using it, the tool needs an update to report it.

### When the layout changes

If the reply has fewer groups than expected or the groups cannot be decoded,
the tool prints a warning and sets `regions` to `null`. Single entries with a
value outside 0 to 1 are skipped. Map selections and modes do not depend on
any of this and are still returned.
