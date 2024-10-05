local m = {}

function m.on_round_started(round)
	print("new round!")
	return {}
end

function m.on_tick(tick)
	if tick % 10 == 0 then
		return { me.move_right(3.5), me.attack(0.5), me.turn_head(0.04) }
	else
		return { me.turn_arms(-0.01) }
	end
end

function m.on_enemy_seen(name, x, y)
	print("saw enemy: " .. name .. " at (" .. x .. ", " .. y .. ")")
	return { me.turn_arms(0.01) }
end

return m
