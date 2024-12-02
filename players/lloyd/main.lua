local m = {}

local s = nil

function m.on_tick(tick, state)
   s = state
   return { me.turn(0.01) }
end

function m.on_death()
   me.log("[lloyd] I'm dead, killed by ?????")
end

function m.on_hit_by(id)
   me.log("HIT OH NO " .. id)
end

function m.on_enemy_seen(name, p)
   angle = math.atan(p.y - s.y, p.x - s.x) + math.pi / 2
   a = utils.normalize_relative_angle(angle - s.heading)
   if math.abs(a) < 0.02 and s.attack_cooldown == 0 then
      me.log("[lloyd] angle = " .. a)
      return { me.attack() }
   end
end

return m
