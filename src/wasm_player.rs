use std::path::Path;

use exports::luarena::player::handlers::{self, Command, Movement, MovementDirection};

use crate::{math_utils, player};

wasmtime::component::bindgen!("player");

struct MyState {
    ctx: wasmtime_wasi::WasiCtx,
    table: wasmtime_wasi::ResourceTable,
    hp: player::ReadableFromImpl<f32>,
    pos: player::ReadableFromImpl<math_utils::Point>,
    heading: player::ReadableFromImpl<f32>,
    head_heading: player::ReadableFromImpl<f32>,
    arms_heading: player::ReadableFromImpl<f32>,
    attack_cooldown: player::ReadableFromImpl<u8>,
    intent: player::ReadableFromImpl<player::Intent>,
}

pub struct WasmImpl {
    bindings: Player,
    store: wasmtime::Store<MyState>,
}

impl luarena::player::me::Host for MyState {
    fn hp(&mut self) -> f32 {
        *self.hp.read().unwrap()
    }

    fn x(&mut self) -> f32 {
        self.pos.read().unwrap().x
    }

    fn y(&mut self) -> f32 {
        self.pos.read().unwrap().y
    }

    fn heading(&mut self) -> f32 {
        *self.heading.read().unwrap()
    }

    fn head_heading(&mut self) -> f32 {
        *self.head_heading.read().unwrap()
    }

    fn arms_heading(&mut self) -> f32 {
        *self.arms_heading.read().unwrap()
    }

    fn attack_cooldown(&mut self) -> u8 {
        *self.attack_cooldown.read().unwrap()
    }

    fn turn_remaining(&mut self) -> f32 {
        self.intent.read().unwrap().turn_angle
    }

    fn head_turn_remaining(&mut self) -> f32 {
        self.intent.read().unwrap().turn_head_angle
    }

    fn arms_turn_remaining(&mut self) -> f32 {
        self.intent.read().unwrap().turn_arms_angle
    }
}

impl WasmImpl {
    pub fn load(
        component_file: &Path,
        player_state: &player::State,
        intent: &player::ReadableFromImpl<player::Intent>,
    ) -> Result<Self, AddWasmPlayerError> {
        let engine = wasmtime::Engine::default();
        let component = wasmtime::component::Component::from_file(&engine, component_file)?;
        let mut linker = wasmtime::component::Linker::new(&engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)?;
        Player::add_to_linker(&mut linker, |state: &mut MyState| state)?;
        let mut builder = wasmtime_wasi::WasiCtxBuilder::new();
        let mut store = wasmtime::Store::new(
            &engine,
            MyState {
                ctx: builder.build(),
                table: wasmtime_wasi::ResourceTable::new(),
                hp: player_state.hp.clone(),
                pos: player_state.pos.clone(),
                heading: player_state.heading.clone(),
                head_heading: player_state.head_heading.clone(),
                arms_heading: player_state.arms_heading.clone(),
                attack_cooldown: player_state.attack_cooldown.clone(),
                intent: intent.clone(),
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

impl From<&MovementDirection> for player::MovementDirection {
    fn from(value: &MovementDirection) -> Self {
        match value {
            MovementDirection::Forward => player::MovementDirection::Forward,
            MovementDirection::Backward => player::MovementDirection::Backward,
            MovementDirection::Left => player::MovementDirection::Left,
            MovementDirection::Right => player::MovementDirection::Right,
        }
    }
}

impl From<&Command> for player::Command {
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

impl From<wasmtime::Error> for player::EventError {
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
        println!("hellloooooo: {msg}");
    }
}

impl From<&math_utils::Point> for handlers::Point {
    fn from(p: &math_utils::Point) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl From<Vec<Command>> for player::Commands {
    fn from(cmds: Vec<Command>) -> Self {
        let mut res = vec![];
        for cmd in cmds.iter() {
            let cmd: player::Command = cmd.into();
            res.push(cmd);
        }
        player::Commands::from(res)
    }
}

impl player::Impl for WasmImpl {
    fn on_event(&mut self, event: &player::Event) -> Result<player::Commands, player::EventError> {
        match event {
            player::Event::Tick(tick) => {
                let commands = self
                    .bindings
                    .luarena_player_handlers()
                    .call_on_tick(&mut self.store, *tick)?;
                Ok(player::Commands::from(commands))
            }
            player::Event::RoundStarted(round) => {
                let commands = self
                    .bindings
                    .luarena_player_handlers()
                    .call_on_round_started(&mut self.store, *round)?;
                Ok(player::Commands::from(commands))
            }
            player::Event::EnemySeen(enemy, p) => {
                let commands = self.bindings.luarena_player_handlers().call_on_enemy_seen(
                    &mut self.store,
                    enemy,
                    p.into(),
                )?;
                Ok(player::Commands::from(commands))
            }
            player::Event::Death => {
                self.bindings
                    .luarena_player_handlers()
                    .call_on_death(&mut self.store)?;
                Ok(player::Commands::none())
            }
            player::Event::HitBy(enemy) => {
                let commands = self
                    .bindings
                    .luarena_player_handlers()
                    // FIXME: enemy
                    .call_on_hit_by(&mut self.store, &enemy.to_string())?;
                Ok(player::Commands::from(commands))
            }
            player::Event::AttackHit(enemy, p) => {
                let commands = self.bindings.luarena_player_handlers().call_on_attack_hit(
                    &mut self.store,
                    &enemy.to_string(),
                    p.into(),
                )?;
                Ok(player::Commands::from(commands))
            }
            player::Event::EnemyDied(enemy_id) => {
                let commands = self
                    .bindings
                    .luarena_player_handlers()
                    .call_on_enemy_died(&mut self.store, &enemy_id.to_string())?;
                Ok(player::Commands::from(commands))
            }
        }
    }
}
