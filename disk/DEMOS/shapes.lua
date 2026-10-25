-- shapes: every Tier 1 primitive, once, so you can see what they look like.
-- argon: resolution = "classic"
alt_screen()
cls(rgb(20, 24, 40))
local white = rgb(255, 255, 255)

pixel(20, 20, white)
draw_text(30, 14, "pixel", white)

line(20, 50, 120, 70, rgb(255, 100, 100))
line_width(4)
line(20, 80, 120, 100, rgb(255, 160, 100))
line_width(1)
draw_text(130, 70, "line, line_width", white)

rect(20, 120, 80, 50, rgb(100, 255, 100))
fill_rect(120, 120, 80, 50, rgb(60, 180, 60))
draw_text(210, 140, "rect, fill_rect", white)

circle(60, 220, 30, rgb(100, 180, 255))
fill_circle(160, 220, 30, rgb(60, 120, 220))
draw_text(210, 214, "circle, fill_circle", white)

ellipse(60, 300, 40, 20, rgb(255, 120, 255))
fill_ellipse(160, 300, 40, 20, rgb(200, 80, 200))
draw_text(210, 294, "ellipse, fill_ellipse", white)

triangle(360, 20, 440, 20, 400, 80, rgb(255, 255, 120))
fill_triangle(460, 20, 540, 20, 500, 80, rgb(220, 220, 60))
draw_text(360, 90, "triangle, fill_triangle", white)

polygon({ 380, 130, 430, 110, 470, 150, 440, 190, 390, 180 }, rgb(120, 255, 220))
fill_polygon({ 490, 130, 540, 110, 580, 150, 550, 190, 500, 180 }, rgb(60, 200, 170))
draw_text(360, 200, "polygon, fill_polygon", white)

blend("add")
fill_circle(420, 280, 40, rgb(255, 0, 0))
fill_circle(460, 280, 40, rgb(0, 255, 0))
fill_circle(440, 310, 40, rgb(0, 0, 255))
blend("alpha")
draw_text(360, 350, "blend('add')", white)

clip(20, 350, 300, 40)
for i = 0, 30 do fill_circle(20 + i * 12, 370, 25, hsv(i * 12, 1, 1)) end
clip()
draw_text(24, 362, "clip(...)", rgb(0, 0, 0))

draw_text(360, 376, "Press any key", rgb(160, 160, 200))
wait_key()
