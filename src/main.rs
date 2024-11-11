use std::{fmt::Display, path::Path};

use exports::luarena::player::handlers::{Movement, MovementDirection, PlayerCommand, Point};
use wasmtime::component::bindgen;
use wasmtime_wasi::{add_to_linker_sync, ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};

mod game;
mod math_utils;
mod render;
mod settings;

bindgen!("player");

impl Display for MovementDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MovementDirection::Forward => write!(f, "forward"),
            MovementDirection::Backward => write!(f, "backward"),
            MovementDirection::Left => write!(f, "left"),
            MovementDirection::Right => write!(f, "right"),
        }
    }
}

impl Display for PlayerCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerCommand::Move(movement) => {
                write!(f, "move: {}, {}", movement.direction, movement.distance)
            }
            PlayerCommand::Attack => write!(f, "attack"),
            PlayerCommand::Turn(angle) => write!(f, "turn {angle}"),
            PlayerCommand::TurnHead(angle) => write!(f, "turn head {angle}"),
            PlayerCommand::TurnArms(angle) => write!(f, "turn arms {angle}"),
        }
    }
}

struct MyState {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl WasiView for MyState {
    fn table(&mut self) -> &mut wasmtime_wasi::ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}

fn main() -> wasmtime::Result<()> {
    let engine = wasmtime::Engine::default();
    let component = wasmtime::component::Component::from_file(&engine, Path::new("comp.wasm"))?;
    let mut linker = wasmtime::component::Linker::<MyState>::new(&engine);
    add_to_linker_sync(&mut linker)?;

    let mut builder = WasiCtxBuilder::new();
    let mut store = wasmtime::Store::new(
        &engine,
        MyState {
            ctx: builder.build(),
            table: ResourceTable::new(),
        },
    );
    let bindings = Player::instantiate::<MyState>(&mut store, &component, &linker)?;
    let res = bindings
        .luarena_player_handlers()
        .call_on_enemy_seen(&mut store, Point { x: 170.1, y: -2.0 })?;
    for cmd in res.iter() {
        println!("command: {cmd}");
    }
    Ok(())
}
