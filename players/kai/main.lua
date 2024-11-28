local m = {}

local dir = 1
local locked = false

function m.on_tick(n)
   if locked then
      return { me.turn_head(-me.head_heading()) }
   elseif me.head_turn_remaining() == 0 then
      dir = -dir
      return { me.turn_head(dir * math.pi) }
   end
end

function m.on_round_started(n)
   me.log("on round started: " .. n)
   locked = false
end

function m.on_death()
   me.log("I'm dead")
end

function m.on_attack_hit(name, p)
   me.log("Gotcha, " .. name)
end

function m.on_hit_by()
   me.log("nooooo")
end

function m.on_enemy_seen(name, p)
   locked = true
   angle = math.atan(p.y - me.y(), p.x - me.x()) + math.pi / 2
   a = utils.normalize_relative_angle(angle - me.heading())
   res = { me.turn(a) }
   if me.turn_remaining() < 0.05 and math.abs(a) < 0.05 and me.attack_cooldown() == 0 then
      me.log("shooting")
      table.insert(res, me.attack())
   end
   return res
end

function m.on_enemy_death(enemy)
   me.log("enemy died: " .. enemy)
end

return m
