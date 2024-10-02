local m = {}

function m.on_tick(tick)
	-- print("Tick: " .. tick)
	-- print("  Current x: " .. me.x())
	-- print("  Current y: " .. me.y())
	if tick < 20 then
		return { me.move_right(3.5) }
	elseif tick > 200 then
		return { me.move_backward(3.5) }
    else
		return {}
	end
end

function m.on_enemy_seen(x, y)
	return {}
end

return m
