local m = {}

function m.on_tick(tick)
	return { me.move_left(10) }
end

return m
