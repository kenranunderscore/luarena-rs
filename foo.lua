m = {}

function m.on_tick(tick)
   -- print("Tick: " .. tick)
   -- print("  Current x: " .. me.x())
   -- print("  Current y: " .. me.y())
   return { tag = "move" }
end

return m
