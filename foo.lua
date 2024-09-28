m = {}

function m.on_tick(tick)
   print("Tick: " .. tick)
   print("  Current x: " .. me.x())
   print("  Current y: " .. me.y())
   return tick + 1
end

return m
