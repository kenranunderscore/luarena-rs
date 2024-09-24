use mlua::prelude::*;
use raylib::prelude::*;

fn _main_lua() -> LuaResult<()> {
    let l = Lua::new();
    let mt = l.create_table()?;
    mt.set(1, "one")?;
    mt.set("two", 2)?;
    l.globals().set("mmm", mt)?;
    l.load("for k,v in pairs(mmm) do print(k,v) end").exec()?;
    Ok(())
}

fn main() {
    let (mut rl, thread) = raylib::init().size(400, 400).title("hello world").build();

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::GRAY);
        d.draw_text("HELLLOOOOOO", 10, 10, 20, Color::BLACK);
    }
}
