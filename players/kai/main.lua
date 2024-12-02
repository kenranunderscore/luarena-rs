local m = {}

local dir = 1
local locked = false
local my_state = nil

function m.on_tick(n, state)
   my_state = state
   if locked then
      return { me.turn_head(-my_state.head_heading) }
   elseif my_state.head_turn_remaining == 0 then
      dir = -dir
      return { me.turn_head(dir * math.pi) }
   end
end

function m.on_round_started(n)
   me.log("on round started: " .. n)
   locked = false
end

function m.on_round_ended(winner)
   me.log("round has ended")
   if winner then
      me.log("congrats " .. winner)
   end
end

function m.on_round_drawn()
   me.log("bahhhh, boring draw")
end

function m.on_round_won()
   me.log("wooot, I won? awesome!")
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
   angle = math.atan(p.y - my_state.y, p.x - my_state.x) + math.pi / 2
   a = utils.normalize_relative_angle(angle - my_state.heading)
   res = { me.turn(a) }
   if my_state.turn_remaining < 0.05 and math.abs(a) < 0.05 and my_state.attack_cooldown == 0 then
      me.log("shooting")
      table.insert(res, me.attack())
   end
   return res
end

function m.on_enemy_death(enemy)
   me.log("enemy died: " .. enemy)
end

return m
