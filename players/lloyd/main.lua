local m = {}

function m.on_tick(tick)
	return { me.turn(0.01) }
end

return m
