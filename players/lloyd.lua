local m = {}

function m.on_tick(tick)
	return { me.move(-1) }
end

return m
