local m = {}

function m.on_round_started(round)
	print("new round!")
    return {}
end

function m.on_tick(tick)
	-- print("Tick: " .. tick)
	-- print("  Current x: " .. me.x())
	-- print("  Current y: " .. me.y())
	if tick % 20 == 0 then
		print("shooting!!!!")
		return { me.move_right(3.5), me.attack(0.5) }
	else
		return {}
	end
end

function m.on_enemy_seen(name, x, y)
	print("saw enemy: " .. name .. " at (" .. x .. ", " .. y .. ")")
	return { me.turn_arms(0.01) }
end

return m
