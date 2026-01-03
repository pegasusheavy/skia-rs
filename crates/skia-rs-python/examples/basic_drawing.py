#!/usr/bin/env python3
"""
Basic drawing example for skia-rs Python bindings.

This example demonstrates:
- Creating a surface
- Drawing basic shapes (rectangles, circles, lines)
- Using different paint styles (fill, stroke)
- Working with paths
- Exporting to PNG (requires PIL/Pillow)
"""

import skia_rs

# Optional: for saving to PNG
try:
    import numpy as np
    from PIL import Image
    HAS_PIL = True
except ImportError:
    HAS_PIL = False
    print("Note: PIL/Pillow not installed. Cannot save PNG.")


def main():
    # Create a 800x600 surface
    width, height = 800, 600
    surface = skia_rs.Surface(width, height)

    # Clear with white background
    surface.clear(skia_rs.Colors.WHITE)

    # Create a filled rectangle paint
    fill_paint = skia_rs.Paint()
    fill_paint.color = skia_rs.rgb(100, 150, 200)  # Light blue
    fill_paint.anti_alias = True

    # Draw a filled rectangle
    surface.draw_rect(50, 50, 250, 200, fill_paint)

    # Create a stroke paint
    stroke_paint = skia_rs.Paint()
    stroke_paint.style = "stroke"
    stroke_paint.stroke_width = 4.0
    stroke_paint.color = skia_rs.Colors.RED
    stroke_paint.anti_alias = True

    # Draw a stroked circle
    surface.draw_circle(400, 150, 80, stroke_paint)

    # Draw an oval
    oval_paint = skia_rs.Paint()
    oval_paint.color = skia_rs.argb(180, 0, 200, 100)  # Semi-transparent green
    surface.draw_oval(500, 50, 750, 200, oval_paint)

    # Draw lines
    line_paint = skia_rs.Paint()
    line_paint.style = "stroke"
    line_paint.stroke_width = 2.0
    line_paint.color = skia_rs.Colors.BLUE

    for i in range(10):
        y = 250 + i * 10
        surface.draw_line(50, y, 350, y + 50, line_paint)

    # Build and draw a path (a star shape)
    builder = skia_rs.PathBuilder()
    cx, cy = 600, 400
    outer_r, inner_r = 100, 40

    import math
    for i in range(5):
        angle_outer = math.radians(i * 72 - 90)
        angle_inner = math.radians(i * 72 + 36 - 90)

        ox = cx + outer_r * math.cos(angle_outer)
        oy = cy + outer_r * math.sin(angle_outer)
        ix = cx + inner_r * math.cos(angle_inner)
        iy = cy + inner_r * math.sin(angle_inner)

        if i == 0:
            builder.move_to(ox, oy)
        else:
            builder.line_to(ox, oy)
        builder.line_to(ix, iy)
    builder.close()

    star_path = builder.build()

    star_paint = skia_rs.Paint()
    star_paint.color = skia_rs.Colors.YELLOW
    surface.draw_path(star_path, star_paint)

    # Stroke the star outline
    star_stroke = skia_rs.Paint()
    star_stroke.style = "stroke"
    star_stroke.stroke_width = 2.0
    star_stroke.color = skia_rs.rgb(200, 150, 0)  # Dark yellow
    surface.draw_path(star_path, star_stroke)

    # Draw a rounded rectangle
    rounded_paint = skia_rs.Paint()
    rounded_paint.color = skia_rs.argb(200, 150, 100, 200)  # Purple

    builder2 = skia_rs.PathBuilder()
    builder2.add_round_rect(50, 350, 300, 550, 20, 20)
    surface.draw_path(builder2.build(), rounded_paint)

    print(f"Created {width}x{height} image")
    print(f"Star path bounds: {star_path.bounds()}")
    print(f"Star contains center: {star_path.contains(600, 400)}")

    # Save to PNG if PIL is available
    if HAS_PIL:
        pixels = surface.pixels()
        arr = np.frombuffer(pixels, dtype=np.uint8).reshape(height, width, 4)
        img = Image.fromarray(arr, mode='RGBA')
        img.save('python_drawing.png')
        print("Saved to python_drawing.png")


if __name__ == "__main__":
    main()
