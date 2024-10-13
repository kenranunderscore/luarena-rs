local m = {}

function m.on_tick(tick)
   return { me.turn(0.01) }
end

function m.on_death()
   print("[lloyd] I'm dead, killed by ?????")
end

function m.on_hit_by(id)
   print("HIT OH NO " .. id)
end

function m.on_enemy_seen(name, x, y)
   angle = math.atan2(y - me.y(), x - me.x()) + math.pi / 2
   a = utils.normalize_relative_angle(angle - me.heading())
   if math.abs(a) < 0.02 and me.attack_cooldown() == 0 then
      print("[lloyd] angle = " .. a)
      return { me.attack() }
   end
end

return m
