m = {}

function m.on_tick(tick)
   print("Tick: " .. tick)
   return tick + 1
end

return m
