use std::path::Path;

use exports::luarena::player::handlers::{Movement, MovementDirection, PlayerCommand};

use crate::player;

wasmtime::component::bindgen!("player");

struct MyState {
    ctx: wasmtime_wasi::WasiCtx,
    table: wasmtime_wasi::ResourceTable,
    hp: player::ReadableFromImpl<f32>,
}

pub struct WasmImpl {
    bindings: Player,
    store: wasmtime::Store<MyState>,
}

impl luarena::player::me::Host for MyState {
    fn hp(&mut self) -> f32 {
        *self.hp.read().unwrap()
    }
}

impl WasmImpl {
    pub fn load(
        component_file: &Path,
        player_state: &player::State,
        _intent: &player::ReadableFromImpl<player::Intent>,
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

impl From<MovementDirection> for player::MovementDirection {
    fn from(value: MovementDirection) -> Self {
        match value {
            MovementDirection::Forward => player::MovementDirection::Forward,
            MovementDirection::Backward => player::MovementDirection::Backward,
            MovementDirection::Left => player::MovementDirection::Left,
            MovementDirection::Right => player::MovementDirection::Right,
        }
    }
}

impl From<PlayerCommand> for player::Command {
    fn from(value: PlayerCommand) -> Self {
        match value {
            PlayerCommand::Move(Movement {
                direction,
                distance,
            }) => Self::Move(direction.into(), distance),
            PlayerCommand::Attack => Self::Attack,
            PlayerCommand::Turn(angle) => Self::Turn(angle),
            PlayerCommand::TurnHead(angle) => Self::TurnHead(angle),
            PlayerCommand::TurnArms(angle) => Self::TurnArms(angle),
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

impl player::Impl for WasmImpl {
    fn on_event(
        &mut self,
        event: &player::Event,
    ) -> Result<player::Commands, player::EventError> {
        match event {
            player::Event::Tick(tick) => {
                let commands = self
                    .bindings
                    .luarena_player_handlers()
                    .call_on_tick(&mut self.store, *tick)?;
                // FIXME: iter? how?
                let mut res = vec![];
                for cmd in commands {
                    let cmd: player::Command = cmd.into();
                    res.push(cmd);
                }
                Ok(player::Commands::from(res))
            }
            _ => Ok(player::Commands::none()),
        }
    }
}
