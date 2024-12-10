use std::path::Path;

use exports::luarena::player::handlers::{self, Command, Movement, MovementDirection};

use super::meta;
use crate::math_utils;

wasmtime::component::bindgen!("player");

struct MyState {
    ctx: wasmtime_wasi::WasiCtx,
    table: wasmtime_wasi::ResourceTable,
    display_name: String,
}

pub struct WasmImpl {
    bindings: Player,
    store: wasmtime::Store<MyState>,
}

impl WasmImpl {
    pub fn load(player_dir: &Path, meta: &meta::Meta) -> Result<Self, AddWasmPlayerError> {
        let engine = wasmtime::Engine::default();
        let file = player_dir.join(&meta.entrypoint);
        let component = wasmtime::component::Component::from_file(&engine, file)?;
        let mut linker = wasmtime::component::Linker::new(&engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)?;
        Player::add_to_linker(&mut linker, |state: &mut MyState| state)?;
        let mut builder = wasmtime_wasi::WasiCtxBuilder::new();
        let mut store = wasmtime::Store::new(
            &engine,
            MyState {
                ctx: builder.build(),
                table: wasmtime_wasi::ResourceTable::new(),
                display_name: meta.display_name(),
            },
        );
        let bindings = Player::instantiate::<MyState>(&mut store, &component, &linker)?;
        Ok(Self { bindings, store })
    }
}

impl wasmtime_wasi::WasiView for MyState {
    fn table(&mut self) -> &mut wasmtime_wasi::ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
        &mut self.ctx
    }
}

impl From<&MovementDirection> for super::MovementDirection {
    fn from(value: &MovementDirection) -> Self {
        match value {
            MovementDirection::Forward => super::MovementDirection::Forward,
            MovementDirection::Backward => super::MovementDirection::Backward,
            MovementDirection::Left => super::MovementDirection::Left,
            MovementDirection::Right => super::MovementDirection::Right,
        }
    }
}

impl From<&Command> for super::Command {
    fn from(value: &Command) -> Self {
        match value {
            Command::Move(Movement {
                direction,
                distance,
            }) => Self::Move(direction.into(), *distance),
            Command::Attack => Self::Attack,
            Command::Turn(angle) => Self::Turn(*angle),
            Command::TurnHead(angle) => Self::TurnHead(*angle),
            Command::TurnArms(angle) => Self::TurnArms(*angle),
        }
    }
}

impl From<wasmtime::Error> for super::EventError {
    fn from(value: wasmtime::Error) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

pub struct AddWasmPlayerError {
    pub message: String,
}

impl From<wasmtime::Error> for AddWasmPlayerError {
    fn from(value: wasmtime::Error) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

impl PlayerImports for MyState {
    fn log(&mut self, msg: String) {
        super::log_msg(&self.display_name, &msg);
    }
}

impl From<&math_utils::Point> for handlers::Point {
    fn from(p: &math_utils::Point) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl From<Vec<Command>> for super::Commands {
    fn from(cmds: Vec<Command>) -> Self {
        let mut res = vec![];
        for cmd in cmds.iter() {
            let cmd: super::Command = cmd.into();
            res.push(cmd);
        }
        super::Commands::from(res)
    }
}

impl From<&super::CurrentPlayerState> for handlers::PlayerState {
    fn from(value: &super::CurrentPlayerState) -> Self {
        handlers::PlayerState {
            x: value.x,
            y: value.y,
            hp: value.hp,
            heading: value.heading,
            head_heading: value.head_heading,
            arms_heading: value.arms_heading,
            attack_cooldown: value.attack_cooldown,
            turn_remaining: value.turn_remaining,
            head_turn_remaining: value.head_turn_remaining,
            arms_turn_remaining: value.arms_turn_remaining,
        }
    }
}

impl super::Impl for WasmImpl {
    fn on_event(&mut self, event: &super::Event) -> Result<super::Commands, super::EventError> {
        match event {
            super::Event::Tick(tick, state) => {
                let commands = self.bindings.luarena_player_handlers().call_on_tick(
                    &mut self.store,
                    *tick,
                    state.into(),
                )?;
                Ok(super::Commands::from(commands))
            }
            super::Event::RoundStarted(round) => {
                let commands = self
                    .bindings
                    .luarena_player_handlers()
                    .call_on_round_started(&mut self.store, *round)?;
                Ok(super::Commands::from(commands))
            }
            super::Event::EnemySeen(enemy, p) => {
                let commands = self.bindings.luarena_player_handlers().call_on_enemy_seen(
                    &mut self.store,
                    enemy,
                    p.into(),
                )?;
                Ok(super::Commands::from(commands))
            }
            super::Event::Death => {
                self.bindings
                    .luarena_player_handlers()
                    .call_on_death(&mut self.store)?;
                Ok(super::Commands::none())
            }
            super::Event::HitBy(enemy) => {
                let commands = self
                    .bindings
                    .luarena_player_handlers()
                    // FIXME: enemy
                    .call_on_hit_by(&mut self.store, &enemy.name.to_string())?;
                Ok(super::Commands::from(commands))
            }
            super::Event::AttackHit(enemy, p) => {
                let commands = self.bindings.luarena_player_handlers().call_on_attack_hit(
                    &mut self.store,
                    &enemy.name.to_string(),
                    p.into(),
                )?;
                Ok(super::Commands::from(commands))
            }
            super::Event::EnemyDied(enemy_id) => {
                let commands = self
                    .bindings
                    .luarena_player_handlers()
                    .call_on_enemy_died(&mut self.store, &enemy_id.to_string())?;
                Ok(super::Commands::from(commands))
            }
            super::Event::RoundEnded(opt_winner) => {
                self.bindings
                    .luarena_player_handlers()
                    .call_on_round_ended(
                        &mut self.store,
                        opt_winner.as_ref().map(|m| m.name.as_str()),
                    )?;
                Ok(super::Commands::none())
            }
            super::Event::RoundWon => {
                self.bindings
                    .luarena_player_handlers()
                    .call_on_round_won(&mut self.store)?;
                Ok(super::Commands::none())
            }
            super::Event::RoundDrawn => {
                self.bindings
                    .luarena_player_handlers()
                    .call_on_round_drawn(&mut self.store)?;
                Ok(super::Commands::none())
            }
        }
    }
}
