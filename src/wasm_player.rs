use std::path::Path;

use exports::luarena::player::handlers::{Movement, MovementDirection, PlayerCommand};

use crate::player;

wasmtime::component::bindgen!("player");

pub struct WasmImpl {
    bindings: Player,
    store: wasmtime::Store<MyState>,
}

impl WasmImpl {
    pub fn load(component_file: &Path) -> Option<Self> {
        let engine = wasmtime::Engine::default();
        let component = wasmtime::component::Component::from_file(&engine, component_file).ok()?;
        let mut linker = wasmtime::component::Linker::<MyState>::new(&engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker).ok()?;
        let mut builder = wasmtime_wasi::WasiCtxBuilder::new();
        let mut store = wasmtime::Store::new(
            &engine,
            MyState {
                ctx: builder.build(),
                table: wasmtime_wasi::ResourceTable::new(),
            },
        );
        let bindings = Player::instantiate::<MyState>(&mut store, &component, &linker).ok()?;
        Some(Self { bindings, store })
    }
}

struct MyState {
    ctx: wasmtime_wasi::WasiCtx,
    table: wasmtime_wasi::ResourceTable,
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

impl From<PlayerCommand> for player::PlayerCommand {
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

impl From<wasmtime::Error> for player::PlayerEventError {
    fn from(value: wasmtime::Error) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

impl player::PlayerImpl for WasmImpl {
    fn on_event(
        &mut self,
        event: &player::PlayerEvent,
    ) -> Result<player::PlayerCommands, player::PlayerEventError> {
        match event {
            player::PlayerEvent::Tick(tick) => {
                let commands = self
                    .bindings
                    .luarena_player_handlers()
                    .call_on_tick(&mut self.store, *tick)?;
                // FIXME: iter? how?
                let mut res = vec![];
                for cmd in commands {
                    let cmd: player::PlayerCommand = cmd.into();
                    res.push(cmd);
                }
                Ok(player::PlayerCommands::from(res))
            }
            _ => Ok(player::PlayerCommands::none()),
        }
    }
}
