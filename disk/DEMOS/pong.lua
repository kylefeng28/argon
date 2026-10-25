-- pong: up/down moves your paddle, the computer plays the right side.
alt_screen()
local w, h = screen_size()
local pw, ph = h // 40, h * 0.15
local left = { x = 20, y = h / 2 - ph / 2, score = 0 }
local right = { x = w - 30, y = h / 2 - ph / 2, score = 0 }
local ball = {}

local function serve(dir)
  ball.x, ball.y = w / 2, h / 2
  ball.dx = 4 * dir
  ball.dy = random(-3, 3)
  if ball.dy == 0 then ball.dy = 1 end
end
serve(1)

local function hit(p)
  return ball.x + 4 > p.x and ball.x - 4 < p.x + pw and ball.y + 4 > p.y and ball.y - 4 < p.y + ph
end

while true do
  cls(rgb(10, 10, 10))

  if button("up") then left.y = left.y - 5 end
  if button("down") then left.y = left.y + 5 end
  left.y = clamp(left.y, 0, h - ph)

  -- Computer: follow the ball, but lazily.
  local target = ball.y - ph / 2
  right.y = lerp(right.y, target, 0.08)
  right.y = clamp(right.y, 0, h - ph)

  ball.x = ball.x + ball.dx
  ball.y = ball.y + ball.dy
  if ball.y < 4 or ball.y > h - 4 then ball.dy = -ball.dy end
  if hit(left) and ball.dx < 0 then
    ball.dx = -ball.dx * 1.05
    ball.dy = ball.dy + (ball.y - (left.y + ph / 2)) / 10
  end
  if hit(right) and ball.dx > 0 then
    ball.dx = -ball.dx * 1.05
    ball.dy = ball.dy + (ball.y - (right.y + ph / 2)) / 10
  end
  if ball.x < 0 then right.score = right.score + 1 serve(1) end
  if ball.x > w then left.score = left.score + 1 serve(-1) end

  for y = 0, h, 20 do fill_rect(w / 2 - 1, y, 2, 10, rgb(60, 60, 60)) end
  fill_rect(left.x, left.y, pw, ph, rgb(255, 255, 255))
  fill_rect(right.x, right.y, pw, ph, rgb(255, 255, 255))
  fill_rect(ball.x - 4, ball.y - 4, 8, 8, rgb(255, 230, 0))
  draw_text(w / 2 - 60, 10, tostring(left.score), rgb(255, 255, 255), 3)
  draw_text(w / 2 + 40, 10, tostring(right.score), rgb(255, 255, 255), 3)
  flip()
end
