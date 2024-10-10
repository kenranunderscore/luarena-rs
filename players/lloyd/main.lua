local m = {}

function m.on_tick(tick)
	return { me.turn(0.01) }
end

function m.on_death()
	print("[lloyd] I'm dead, killed by ?????")
    return {}
end

function m.on_hit_by(id)
	print("HIT OH NO " .. id)
    return {}
end

return m
