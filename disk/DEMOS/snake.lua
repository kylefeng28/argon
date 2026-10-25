-- snake: a game on the text grid using put_char. Arrows move, Esc quits.
-- argon: fps = 12
alt_screen()
local cols, rows = text_size()
local snake = { { x = 10, y = 10 }, { x = 9, y = 10 }, { x = 8, y = 10 } }
local dir = { x = 1, y = 0 }
local food = { x = random(1, cols - 2), y = random(2, rows - 2) }
local score = 0
local best = load("best", 0)
local alive = true

local function draw()
  cls(rgb(0, 0, 0))
  text_color(rgb(90, 90, 90))
  for x = 0, cols - 1 do put_char(x, 1, "-") put_char(x, rows - 1, "-") end
  for y = 1, rows - 1 do put_char(0, y, "|") put_char(cols - 1, y, "|") end
  put_char(food.x, food.y, "*", rgb(255, 80, 80))
  for i, s in ipairs(snake) do
    put_char(s.x, s.y, i == 1 and "@" or "o", rgb(80, 255, 80))
  end
  locate(0, 0)
  text_color(rgb(255, 255, 255))
  write(string.format("SNAKE  score %d  best %d", score, best))
end

while alive do
  if button("left") and dir.x == 0 then dir = { x = -1, y = 0 } end
  if button("right") and dir.x == 0 then dir = { x = 1, y = 0 } end
  if button("up") and dir.y == 0 then dir = { x = 0, y = -1 } end
  if button("down") and dir.y == 0 then dir = { x = 0, y = 1 } end

  local head = { x = snake[1].x + dir.x, y = snake[1].y + dir.y }
  if head.x <= 0 or head.x >= cols - 1 or head.y <= 1 or head.y >= rows - 1 then alive = false end
  for _, s in ipairs(snake) do
    if s.x == head.x and s.y == head.y then alive = false end
  end
  if alive then
    table.insert(snake, 1, head)
    if head.x == food.x and head.y == food.y then
      score = score + 1
      food = { x = random(1, cols - 2), y = random(2, rows - 2) }
    else
      table.remove(snake)
    end
  end
  draw()
  flip()
end

if score > best then best = score save("best", best) end
locate(cols / 2 - 10, rows / 2)
text_color(rgb(255, 255, 0), rgb(120, 0, 0))
write("  GAME OVER - score " .. score .. "  ")
text_color(rgb(255, 255, 255))
flip()
wait(0.5)
wait_key()
