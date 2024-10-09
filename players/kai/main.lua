local m = {}

local dir = 1

function m.on_tick(n)
	return { me.turn(10000000) }
	-- if me.head_turn_remaining() == 0 then
	-- 	dir = -dir
	-- 	return { me.turn_head(dir * math.pi) }
	-- end
end

function normalize_relative_angle(angle)
	while angle < -math.pi do
		angle = angle + 2 * math.pi
	end
	while angle >= math.pi do
		angle = angle - 2 * math.pi
	end
	return angle
end

function m.on_enemy_seen(name, x, y)
	print("seeing " .. name)
	angle = math.atan2(y - me.y(), x - me.x()) + math.pi / 2
	a = normalize_relative_angle(angle - me.heading())
    print("angle: " .. a)
	res = {}
	if math.abs(a) < 0.05 then
		print("   shooting now")
		table.insert(res, me.attack())
	end
	return res
end

return m
