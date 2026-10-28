# TreeFall

A **native** Horizon mod (horizon-mixins source mixin): cut one log of a natural
tree with an axe and the whole tree comes down. Every log is broken through the
game's ordinary mining path, so drops, statistics, hunger and axe wear are
exactly as if you had chopped each log yourself (all of that is configurable).

> **Native mods are not sandboxed.** Unlike `.hmod` (Wasm) mods, a native mod is
> Rust source compiled into your own Horizon executable and runs with your user
> rights. Read the source (it is small: `mixins/treefall/`) before building.

## Behaviour

Breaking a block fells a tree when all of this holds:

- the block is a **log**: in the game's `minecraft:logs` block tag (`*_log`,
  `*_wood`, nether `*_stem`/`*_hyphae`, also modded logs added to the tag), but
  **not stripped** (stripped logs are almost always building material); with
  `mushrooms`, also `minecraft:mushroom_stem`;
- its wood is allowed by `woods`;
- the main hand holds what `tool` asks for (by default an **axe**, the
  `minecraft:axes` item tag);
- you are in **survival or adventure** (adventure still obeys `can_break` for
  every log), playing locally or hosting; never in creative/spectator or as a
  client of a remote server;
- the `sneak` rule allows it (by default: **not sneaking**; sneak to chop a
  single log);
- TreeFall is on for you (`/treefall`) and in the configuration.

The tree is every log of the **same wood** (`oak_log` and `oak_wood` count as
one wood, birch next to it does not) connected through the 26 surrounding
blocks, **at or above** the cut block. Logs below the cut stay.

It is felled only if it is a **real tree**: at least `minLeaves` (default **5**)
natural leaves — leaves with `persistent=false`, i.e. grown, not placed by a
player — touch the logs. Nether stems count their crown instead (nether/warped
wart blocks, shroomlight), huge mushrooms their red/brown mushroom-block cap. A
log cabin, a log pillar or a wall decorated with placed leaves is therefore
broken one log at a time, as usual. Groups larger than `maxLogs` (default
**256**; a 2x2 jungle giant is ~130) are not felled at all rather than partially.

The cut block breaks first, then the rest **top-down**, so if felling stops the
part left is a standing stump you can still reach. Felling stops:

- before the tool would drop below `keepDurability` points (default 1: the
  axe is never broken by felling; the cut log itself always breaks, as in vanilla),
- if the held item changes, or a log fails to break.

Leaves are **left to decay naturally** (vanilla). With `breakLeaves` they are
broken right away instead (drops as if broken with the held tool: saplings,
sticks, apples, Fortune applies; no tool wear), but only the leaves vanilla
would let decay: within 6 steps of the felled logs and not within 6 steps of any
other log (leaves next to unloaded chunks are left alone). Nether crowns and
mushroom caps never decay and are never broken.

A single log with nothing else to fell is left to the game (or to Veinminer).

## Commands

| Command | Effect |
|---|---|
| `/treefall` or `/treefall status` | Show whether it is on for you |
| `/treefall on` / `off` / `toggle` | Turn it on/off for you (per player, per world, until restart) |
| `/treefall reload` | Re-read the configuration file (permission level 2) |

LAN guests can use the same commands; each player has their own on/off state.

## Configuration

Set `HORIZON_TREEFALL_CONFIG` to the path of a JSON file before starting the
game. Every key is optional; unknown keys or out-of-range values make the mod
disable itself (it prints `TreeFall disabled: …` to stderr). The defaults:

```json
{
  "enabled": true,
  "tool": "axe",
  "sneak": "disables",
  "maxLogs": 256,
  "minLeaves": 5,
  "breakLeaves": false,
  "breakSpeed": "single",
  "speedFactor": 0.5,
  "maxSlowdown": 20,
  "durability": "perLog",
  "durabilityFactor": 0.5,
  "keepDurability": 1,
  "exhaustion": "perLog",
  "dropsAtStump": false,
  "replant": false,
  "woods": { "allow": [], "deny": [] },
  "mushrooms": false
}
```

| Key | Default | Values |
|---|---|---|
| `enabled` | `true` | Global switch |
| `tool` | `"axe"` | `"axe"` (`minecraft:axes`), `"anyTool"` (any item with a tool component: pickaxes, shovels, hoes, swords, shears...), `"any"` (bare hands too) |
| `sneak` | `"disables"` | `"disables"`: sneak to cut one log. `"enables"`: only sneaking fells. `"ignored"` |
| `maxLogs` | `256` | Largest tree, 1..=4096; bigger log groups are not felled |
| `minLeaves` | `5` | Natural leaves (crown/cap blocks for stems/mushrooms) touching the logs, 0..=1024. `0` fells **any** log structure, cabins included |
| `breakLeaves` | `false` | Also break the leaves that would decay |
| `breakSpeed` | `"single"` | `"single"`: the cut takes as long as one log. `"scaled"`: the cut log takes `1 + (logs - 1) * speedFactor` times longer, at most `maxSlowdown` times |
| `speedFactor` | `0.5` | 0..=10, for `"scaled"` (a 6-log oak: 3.5x) |
| `maxSlowdown` | `20` | 1..=100, for `"scaled"` |
| `durability` | `"perLog"` | `"perLog"`: the game's wear for every log (Unbreaking applies). `"once"`: one use per tree. `"multiplier"`: `ceil(logs * durabilityFactor)` uses per tree. `"none"`: no wear, not even for the cut log |
| `durabilityFactor` | `0.5` | 0..=10, for `"multiplier"` |
| `keepDurability` | `1` | Stop before the tool would have fewer points left. `0` lets felling use the tool up |
| `exhaustion` | `"perLog"` | `"perLog"`: the game's hunger cost per block broken. `"once"`: only the cut log's |
| `dropsAtStump` | `false` | Pop all drops at the cut block instead of where each block was |
| `replant` | `false` | After a complete felling, plant a sapling of that wood (`*_sapling`, `mangrove_propagule`, `crimson_fungus`/`warped_fungus`) from your inventory at the cut block, if the block below can hold it (the game's own `canSurvive`) |
| `woods` | all | `allow`/`deny` lists of wood keys: `"oak"`, `"dark_oak"`, `"crimson"`, `"minecraft:warped"`, `"mushroom"`, modded `"ns:wood"`... An empty `allow` allows all; `deny` wins |
| `mushrooms` | `false` | Fell huge mushrooms: `mushroom_stem` with a red/brown mushroom-block cap |

Why 5 leaves: the smallest vanilla trees (a 4-log oak, an acacia, an azalea
tree) touch 20+ natural leaves around their top log, while leaves that happen to
grow against a wall rarely reach 5.

Why `breakSpeed` defaults to `"single"`: it is today's behaviour, and the
slowdown is applied by the host's game. A LAN guest's own client still predicts
the normal (single-log) time; the host then holds the block back until the
slowed time is up, so for guests the log can briefly reappear before the tree
falls. A guest who mines a log instantly (Efficiency + Haste) is not slowed.
For a single player or the host it behaves like a slower block.

Other notes: `replant` plants one sapling at the cut block, so 2x2 trees (dark
oak, pale oak, mega spruce/jungle) need the other three planted by hand, and
huge mushrooms are not replanted. Toggles are kept per world in memory, not
saved.

## Veinminer

[Veinminer](https://github.com/levzzz5154/horizon-veinminer) also mines logs by
default. Both mods pin the same Horizon and horizon-mixins commits, so they
build together (checked with `horizon-mixins apply`). TreeFall's hooks have
priority 200, above Veinminer's 100; same-site HEAD hooks run highest first and
the first that cancels skips the rest. So:

- a natural tree cut with the configured tool is felled by TreeFall only;
- anything TreeFall declines (sneaking, wrong tool, a log cabin, too few
  leaves, a denied wood) goes on to Veinminer, which may vein-mine those logs
  by its own rules.

For predictable trees use one or the other for logs: either remove the
`*-logs` groups from Veinminer's configuration (`HORIZON_VEINMINER_CONFIG`), or
turn TreeFall off. Note that sneaking does not stop Veinminer by default.

## Build and try

TreeFall is a source bundle; the launcher compiles it into a custom game:

```sh
horizon-launcher native-build <this folder> --version v0.2.94 --trust-native-code
```

`<this folder>` may also be a folder holding several native mods (for example
this one and Veinminer).

## Development

`mixins/treefall/`:

- `horizon-mixin.toml` — two support modules and five cancellable HEAD hooks:
  `Game::break_block` (felling), `hand::Game::destroy_progress`
  (`breakSpeed`), `Game::pop_resource` (`dropsAtStump`),
  `commands::Game::run_command` and `lan_actions::Game::guest_command`
  (commands).
- `runtime.rs` — all decisions (tree finding, leaf thresholds, decaying leaves,
  wear, speed, config, commands) as pure code over a small `Blocks`/`Host`
  trait, with unit tests. It is both this repository's Cargo library and the
  injected `crate::game::treefall` module.
- `native.rs` — the adapter to the game (`crate::game::treefall_native`).
- `mining.rs`, `speed.rs`, `drops.rs`, `command.rs`, `guest-command.rs` — the
  hook helpers.

```sh
cargo test
python tools/pack_hnmod.py   # dist/treefall-<version>.hnmod
```

Pinned: Horizon `83f15fa276c44711276b6ceb5f0fe0c9c534dc66`, horizon-mixins
`0500ff24aa5933fca95a877ef41125353c4b4fe3`, game 26.3.

Original code; no code from other tree-felling mods was used. MIT licensed.
