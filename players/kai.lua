local m = {}

function m.on_tick(tick)
	-- print("Tick: " .. tick)
	-- print("  Current x: " .. me.x())
	-- print("  Current y: " .. me.y())
    return { me.move(3.14), me.turn_head(0.04), me.turn(-0.01), me.turn_arms(-0.02) }
end

function m.on_enemy_seen(x, y)
	return {}
end

return m
