// TreeFall's native adapter for Horizon 83f15fa276c44711276b6ceb5f0fe0c9c534dc66.
// Injected as `crate::game::treefall_native`; binds the game-independent engine
// in `crate::game::treefall` (runtime.rs) to the running game.
use super::treefall::{
    BreakSpeed, Block, Blocks, Command, Config, Host, Kind, MineFlags, Player, Position, Runtime,
    SpeedKey, Tool, plan_tree,
};
use super::{Game, Result};

#[derive(Default)]
struct State {
    runtime: Runtime,
    config: Option<Config>,
}

thread_local! {
    // Per world, on the thread that runs the game (like the game's own state).
    static STATES: std::cell::RefCell<std::collections::BTreeMap<std::path::PathBuf, State>> =
        const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
    // Where the drops of the blocks being felled go (`dropsAtStump`).
    static GATHER: std::cell::Cell<Option<Position>> = const { std::cell::Cell::new(None) };
    // Set while `destroy_progress` runs the game's own computation.
    static MEASURING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
fn with_state<T>(key: &std::path::Path, run: impl FnOnce(&mut State) -> T) -> T {
    struct Lease {
        key: std::path::PathBuf,
        state: State,
    }
    impl Drop for Lease {
        fn drop(&mut self) {
            STATES.with(|states| {
                states
                    .borrow_mut()
                    .insert(self.key.clone(), std::mem::take(&mut self.state));
            });
        }
    }
    let state = STATES.with(|states| states.borrow_mut().remove(key).unwrap_or_default());
    let mut lease = Lease {
        key: key.to_owned(),
        state,
    };
    run(&mut lease.state)
}

fn load_config() -> std::result::Result<Config, String> {
    match std::env::var_os(super::treefall::CONFIG_ENV) {
        Some(path) => {
            let path = std::path::PathBuf::from(path);
            Config::from_json(
                &std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?,
            )
        }
        None => Ok(Config::default()),
    }
}
/// The loaded configuration; a broken file disables the mod (fails closed).
fn config(state: &mut State) -> &Config {
    state.config.get_or_insert_with(|| {
        load_config().unwrap_or_else(|error| {
            eprintln!("TreeFall disabled: {error}");
            Config {
                enabled: false,
                ..Config::default()
            }
        })
    })
}

/// Read-only view of the game: blocks, the acting player and their tool.
struct View<'g, 'r>(&'g Game<'r>);
impl Blocks for View<'_, '_> {
    fn block(&self, p: Position) -> Option<Block> {
        let game = self.0;
        if p[1] < game.dim.min_y() || p[1] > game.dim.max_y() || !game.world.loaded(p[0], p[2]) {
            return None;
        }
        let state = game.world.block(p[0], p[1], p[2]);
        let facts = game.res.facts().get(state);
        let items = game.res.items();
        let name: &str = &facts.name;
        let kind = if game.world.is_air(state) || facts.hardness < 0.0 {
            Kind::Other
        } else if items.block_in_tag("minecraft:logs", name) {
            Kind::Log
        } else if items.block_in_tag("minecraft:leaves", name) {
            // LeavesBlock.PERSISTENT: placed by a player.
            let persistent = game
                .pack
                .assets
                .registry
                .state(state)
                .and_then(|s| s.properties.get("persistent"))
                .map(String::as_str)
                == Some("true");
            Kind::Leaves {
                natural: !persistent,
            }
        } else if items.block_in_tag("minecraft:wart_blocks", name)
            || name == "minecraft:shroomlight"
        {
            Kind::Crown
        } else if name == "minecraft:mushroom_stem" {
            Kind::MushroomStem
        } else if name == "minecraft:red_mushroom_block" || name == "minecraft:brown_mushroom_block"
        {
            Kind::Cap
        } else {
            Kind::Other
        };
        Some(Block {
            name: name.to_owned(),
            state: state.0,
            kind,
        })
    }
}
impl View<'_, '_> {
    fn player(&self, actor: [i32; 4]) -> Player {
        let game = self.0;
        Player {
            actor,
            survival: game.remote.is_none()
                && game.player.is_some()
                && !game.creative
                && !game.spectator(),
            // Sneak is the left shift key; a LAN guest's sneak flag is mapped
            // onto it for their turn (`enter_guest`).
            sneaking: game.keys.contains(&winit::keyboard::KeyCode::ShiftLeft)
                || game.player.as_ref().is_some_and(|p| p.shift_key_down),
        }
    }
    /// `acting_player_uuid` without its `&mut` (the host's UUID is set once a
    /// command or a break has asked for it).
    fn actor(&self) -> [i32; 4] {
        match self.0.lan_acting_uuid {
            Some(guest) => horizon_client::lan::playerdata::uuid_ints(guest),
            None => self.0.level.player_uuid.unwrap_or_default(),
        }
    }
    fn tool(&self) -> Option<Tool> {
        let stack = self.0.held()?;
        let items = self.0.res.items();
        let info = items.get(&stack.item);
        let max = info.map_or(0, |i| i.max_damage);
        let axe = match items.item_tags.get("minecraft:axes") {
            Some(axes) => axes.contains(&*stack.item),
            None => stack.item.ends_with("_axe"),
        };
        Some(Tool {
            item: stack.item.to_string(),
            axe,
            tool: info.is_some_and(|i| !i.tool.is_empty() || i.tool_damage_per_block.is_some()),
            remaining: (max > 0).then_some(max.saturating_sub(stack.damage)),
        })
    }
}

struct Native<'g, 'r> {
    game: &'g mut Game<'r>,
}
impl Blocks for Native<'_, '_> {
    fn block(&self, p: Position) -> Option<Block> {
        View(&*self.game).block(p)
    }
}
impl Host for Native<'_, '_> {
    type Error = Box<dyn std::error::Error>;
    fn player(&mut self) -> Player {
        let actor = self.game.acting_player_uuid();
        View(&*self.game).player(actor)
    }
    fn tool(&self) -> Option<Tool> {
        View(&*self.game).tool()
    }
    fn mine(&mut self, p: Position, flags: MineFlags) -> Result<bool> {
        let before = self.block(p);
        let selected = self.game.selected;
        let held = flags
            .keep_durability
            .then(|| self.game.inventory[selected].clone());
        let exhaustion = flags
            .keep_exhaustion
            .then(|| self.game.player.as_ref().map(|p| p.health.food.exhaustion))
            .flatten();
        // Clear the damage for the break and put it back afterwards.
        if flags.keep_durability
            && let Some(stack) = self.game.inventory[selected].as_mut()
        {
            stack.damage = 0;
        }
        let result = self.game.break_block_as_player(p);
        if let Some(original) = held {
            match (self.game.inventory[selected].as_mut(), original) {
                (Some(now), Some(was)) if now.item == was.item => now.damage = was.damage,
                (None, Some(was)) => self.game.inventory[selected] = Some(was),
                _ => {}
            }
        }
        if let Some(exhaustion) = exhaustion
            && let Some(player) = self.game.player.as_mut()
        {
            player.health.food.exhaustion = exhaustion;
        }
        result?;
        Ok(before.is_some() && self.block(p) != before)
    }
    /// `ItemStack.hurtAndBreak` for the held tool, as `mine_block_with_held` does it.
    fn wear(&mut self, amount: u32) {
        let selected = self.game.selected;
        if amount == 0 {
            return;
        }
        let Some(stack) = self.game.inventory[selected].clone() else {
            return;
        };
        let max = self
            .game
            .res
            .items()
            .get(&stack.item)
            .map_or(0, |i| i.max_damage);
        if max == 0 {
            return;
        }
        self.game.trigger_item_durability(&stack);
        let loss = self.game.durability_loss(&stack, amount);
        if let Some(held) = self.game.inventory[selected].as_mut() {
            held.damage += loss;
            if held.damage >= max {
                self.game.inventory[selected] = None;
            }
        }
    }
    fn gather(&mut self, at: Option<Position>) {
        GATHER.set(at);
    }
    fn replant(&mut self, at: Position, item: &str) -> bool {
        let game = &mut *self.game;
        let Some(state) = game.pack.assets.registry.default_state(item) else {
            return false;
        };
        let slot = (0..game.inventory.len())
            .find(|&i| game.inventory[i].as_ref().is_some_and(|s| &*s.item == item));
        let offhand = game.offhand.as_ref().is_some_and(|s| &*s.item == item);
        if slot.is_none() && !offhand {
            return false;
        }
        let free = game.res.facts().is_air(game.world.block(at[0], at[1], at[2]));
        let survives = free
            && game.with_level(|b, level, ticks| {
                b.can_survive(
                    level,
                    ticks,
                    horizon_core::position::BlockPos::new(at[0], at[1], at[2]),
                    state,
                )
            });
        if !survives || game.level_set(at, state).is_err() {
            return false;
        }
        let stack = match slot {
            Some(i) => &mut game.inventory[i],
            None => &mut game.offhand,
        };
        if let Some(s) = stack.as_mut() {
            s.count = s.count.saturating_sub(1);
            if s.count == 0 {
                *stack = None;
            }
        }
        true
    }
}

impl Game<'_> {
    /// `Game::break_block`'s HEAD: Some when the break fells a tree (the whole
    /// break, the clicked block included, happened here), None to let the
    /// game (and lower-priority hooks) break the block as usual.
    pub(super) fn treefall_break_block(&mut self, pos: Position) -> Option<Result<()>> {
        let key = self.world_root.clone();
        with_state(&key, |state| {
            let config = config(state).clone();
            let plan = state
                .runtime
                .plan(&mut Native { game: self }, &config, pos)
                .ok()?;
            let runtime = &mut state.runtime;
            Some(self.with_player_actor(|game| {
                let result = runtime.fell(&mut Native { game }, &config, &plan);
                GATHER.set(None);
                result.map(|_| ())
            }))
        })
    }

    /// `Game::destroy_progress`'s HEAD (`breakSpeed: "scaled"`): the cut
    /// log's progress per tick divided by the tree's slowdown; None leaves the
    /// game's own value.
    pub(super) fn treefall_destroy_progress(&self, pos: Position) -> Option<f32> {
        if MEASURING.get() {
            return None;
        }
        let view = View(self);
        let slowdown = with_state(&self.world_root, |state| {
            let config = config(state).clone();
            if config.break_speed == BreakSpeed::Single || !config.enabled {
                return 1.0;
            }
            let actor = view.actor();
            let tool = view.tool();
            let key = SpeedKey {
                actor,
                position: pos,
                state: self.world.block(pos[0], pos[1], pos[2]).0,
                item: tool.as_ref().map(|t| t.item.clone()),
            };
            let enabled = state.runtime.enabled(actor);
            state.runtime.slowdown(key, self.world.game_time, || {
                let player = view.player(actor);
                plan_tree(&view, &config, &player, enabled, tool, pos, false)
                    .map_or(1.0, |plan| plan.slowdown(&config))
            })
        });
        if slowdown <= 1.0 {
            return None;
        }
        struct Measuring;
        impl Drop for Measuring {
            fn drop(&mut self) {
                MEASURING.set(false);
            }
        }
        MEASURING.set(true);
        let _measuring = Measuring;
        Some(self.destroy_progress(pos) / slowdown)
    }

    /// `Game::pop_resource`'s HEAD (`dropsAtStump`): while a tree is felled,
    /// pop `stack` at the stump instead; false leaves it where it is.
    pub(super) fn treefall_pop_at_stump(
        &mut self,
        pos: Position,
        stack: &horizon_client::items::ItemStack,
    ) -> bool {
        let Some(stump) = GATHER.get() else {
            return false;
        };
        if stump == pos {
            return false;
        }
        GATHER.set(None);
        self.pop_resource(stump, stack.clone());
        GATHER.set(Some(stump));
        true
    }

    /// `/treefall [status|on|off|toggle|reload]`; None for other commands.
    pub(super) fn treefall_command(&mut self, command: &str) -> Option<bool> {
        let parsed = super::treefall::parse_command(command)?;
        let key = self.world_root.clone();
        with_state(&key, |state| {
            let message = match parsed {
                Err(usage) => {
                    self.command_failure(horizon_client::text::Text::literal(usage));
                    return Some(false);
                }
                Ok(Command::Reload) => {
                    if self.ui.registries.permission < 2 {
                        self.command_failure(horizon_client::text::Text::literal(
                            "TreeFall reload requires permission level 2",
                        ));
                        return Some(false);
                    }
                    match load_config() {
                        Ok(config) => {
                            state.config = Some(config);
                            state.runtime.forget_speed();
                            "TreeFall configuration reloaded".to_owned()
                        }
                        Err(error) => {
                            self.command_failure(horizon_client::text::Text::literal(error));
                            return Some(false);
                        }
                    }
                }
                Ok(command) => {
                    let actor = self.acting_player_uuid();
                    let config = config(state).clone();
                    state.runtime.command(actor, command, &config)
                }
            };
            self.system_message(horizon_client::text::Text::literal(message));
            Some(true)
        })
    }
}
