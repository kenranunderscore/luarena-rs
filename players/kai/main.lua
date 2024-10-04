local m = {}

function m.on_tick(tick)
	-- print("Tick: " .. tick)
	-- print("  Current x: " .. me.x())
	-- print("  Current y: " .. me.y())
	if tick < 20 then
		return { me.move_right(3.5) }
	elseif tick == 100 then
		print("shooting!!!!")
		return { me.attack(0.5) }
	else
		return { me.turn(0.1), me.turn_head(-0.02) }
	end
end

function m.on_enemy_seen(name, x, y)
	print("saw enemy: " .. name .. " at (" .. x .. ", " .. y .. ")")
	return { me.turn_arms(0.01) }
end

return m
