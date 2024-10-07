local m = {}

function m.on_tick(tick)
	return { me.move_left(1), me.turn(0.01) }
end

return m
