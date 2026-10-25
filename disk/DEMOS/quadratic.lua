-- quadratic: the kind of homework program you'd write on a TI-84.
-- Solves a*x^2 + b*x + c = 0.
cls()
print("QUADRATIC SOLVER")
print("================")
print()
while true do
  local a = tonumber(input("a = "))
  local b = tonumber(input("b = "))
  local c = tonumber(input("c = "))
  if a == nil or b == nil or c == nil then
    print("Please enter numbers.")
  elseif a == 0 then
    if b == 0 then
      print("No solution.")
    else
      print(string.format("x = %g", -c / b))
    end
  else
    local d = b * b - 4 * a * c
    if d < 0 then
      local re, im = -b / (2 * a), math.sqrt(-d) / (2 * a)
      print(string.format("x = %g + %gi", re, im))
      print(string.format("x = %g - %gi", re, im))
    elseif d == 0 then
      print(string.format("x = %g", -b / (2 * a)))
    else
      print(string.format("x1 = %g", (-b + math.sqrt(d)) / (2 * a)))
      print(string.format("x2 = %g", (-b - math.sqrt(d)) / (2 * a)))
    end
  end
  print()
  local again = input("Again? (y/n) ")
  if again:lower() ~= "y" then break end
  print()
end
