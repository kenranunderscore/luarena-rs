m = {}

function m.on_tick(tick)
   -- print("Tick: " .. tick)
   -- print("  Current x: " .. me.x())
   -- print("  Current y: " .. me.y())
   return me.move(3.14)
end

return m
