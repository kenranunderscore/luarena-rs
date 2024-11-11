use std::path::Path;

use wasmtime::component::bindgen;

mod game;
mod math_utils;
mod render;
mod settings;

bindgen!({
  inline: r#"
package foo:stringoperations;

interface theint {
    shmup: func(s: string) -> u32;
}

world theworld {
    export theint;
}"#,
});

fn main() -> wasmtime::Result<()> {
    let engine = wasmtime::Engine::default();
    let mut store = wasmtime::Store::new(&engine, ());
    let component =
        wasmtime::component::Component::from_file(&engine, Path::new("wasm_comp_rs.wasm"))?;
    let linker = wasmtime::component::Linker::new(&engine);
    let bindings = Theworld::instantiate(&mut store, &component, &linker)?;
    let res = bindings
        .foo_stringoperations_theint()
        .call_shmup(&mut store, "shmuppppp")?;
    println!("result: {res}");
    Ok(())
}
