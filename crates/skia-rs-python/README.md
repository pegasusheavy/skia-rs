# skia-rs-python

Python bindings for skia-rs, a 100% Rust implementation of Google's Skia 2D graphics library.

## Installation

### From PyPI (when published)

```bash
pip install skia-rs
```

### From Source

```bash
# Install maturin
pip install maturin

# Build and install
cd crates/skia-rs-python
maturin develop --release
```

## Quick Start

```python
import skia_rs

# Create a surface
surface = skia_rs.Surface(800, 600)

# Create a paint
paint = skia_rs.Paint()
paint.color = skia_rs.Colors.RED
paint.anti_alias = True

# Clear and draw
surface.clear(skia_rs.Colors.WHITE)
surface.draw_circle(400, 300, 100, paint)

# Access pixel data
pixels = surface.pixels()  # RGBA bytes
```

## API Reference

### Surface

```python
surface = skia_rs.Surface(width, height)
surface.width       # int: Width in pixels
surface.height      # int: Height in pixels
surface.clear(color)
surface.draw_rect(left, top, right, bottom, paint)
surface.draw_circle(cx, cy, radius, paint)
surface.draw_oval(left, top, right, bottom, paint)
surface.draw_line(x0, y0, x1, y1, paint)
surface.draw_path(path, paint)
surface.draw_point(x, y, paint)
surface.pixels()    # bytes: RGBA pixel data
```

### Paint

```python
paint = skia_rs.Paint()
paint.color = 0xFFFF0000      # ARGB color
paint.style = "fill"          # "fill", "stroke", "stroke_and_fill"
paint.stroke_width = 2.0
paint.anti_alias = True
paint.alpha = 255             # 0-255
paint.set_argb(255, 255, 0, 0)  # Set from components
```

### Path & PathBuilder

```python
builder = skia_rs.PathBuilder()
builder.move_to(0, 0)
builder.line_to(100, 0)
builder.quad_to(150, 50, 100, 100)
builder.cubic_to(50, 150, 0, 150, 0, 100)
builder.close()
builder.add_rect(10, 10, 50, 50)
builder.add_circle(100, 100, 25)
builder.add_oval(0, 0, 100, 50)
builder.add_round_rect(0, 0, 100, 100, 10, 10)

path = builder.build()
path.is_empty()
path.bounds()        # Rect
path.contains(x, y)  # bool
```

### Geometry

```python
# Point
p = skia_rs.Point(10, 20)
p.x, p.y
p.length()
p.normalize()
p1 + p2, p1 - p2, p * 2.0

# Rect
r = skia_rs.Rect(0, 0, 100, 100)
r = skia_rs.Rect.from_xywh(10, 10, 80, 80)
r = skia_rs.Rect.from_wh(100, 100)
r.left, r.top, r.right, r.bottom
r.width, r.height
r.is_empty()
r.contains(50, 50)
r.center()  # Point
r.join(x, y)  # Expand to include point

# Matrix
m = skia_rs.Matrix()           # Identity
m = skia_rs.Matrix.translate(10, 20)
m = skia_rs.Matrix.scale(2, 2)
m = skia_rs.Matrix.rotate(radians)
m = skia_rs.Matrix.rotate_deg(45)
m.concat(other)  # Matrix multiplication
m.invert()       # Optional[Matrix]
m.map_point(x, y)  # Point
```

### Colors

```python
# Color utilities
color = skia_rs.argb(255, 255, 0, 0)  # Red
color = skia_rs.rgb(0, 255, 0)        # Green (opaque)

# Predefined colors
skia_rs.Colors.BLACK
skia_rs.Colors.WHITE
skia_rs.Colors.RED
skia_rs.Colors.GREEN
skia_rs.Colors.BLUE
skia_rs.Colors.YELLOW
skia_rs.Colors.CYAN
skia_rs.Colors.MAGENTA
skia_rs.Colors.TRANSPARENT
```

## Examples

### Drawing Shapes

```python
import skia_rs

surface = skia_rs.Surface(400, 400)
surface.clear(skia_rs.Colors.WHITE)

# Filled rectangle
fill = skia_rs.Paint()
fill.color = skia_rs.rgb(100, 150, 200)
surface.draw_rect(50, 50, 150, 150, fill)

# Stroked circle
stroke = skia_rs.Paint()
stroke.style = "stroke"
stroke.stroke_width = 3
stroke.color = skia_rs.Colors.RED
surface.draw_circle(300, 100, 50, stroke)

# Path
builder = skia_rs.PathBuilder()
builder.move_to(200, 200)
builder.line_to(300, 250)
builder.line_to(250, 350)
builder.close()

path_paint = skia_rs.Paint()
path_paint.color = skia_rs.argb(128, 0, 255, 0)  # Semi-transparent green
surface.draw_path(builder.build(), path_paint)
```

### Using with NumPy/PIL

```python
import skia_rs
import numpy as np
from PIL import Image

# Create and draw
surface = skia_rs.Surface(800, 600)
surface.clear(skia_rs.Colors.WHITE)
# ... draw operations ...

# Convert to NumPy array
pixels = surface.pixels()
arr = np.frombuffer(pixels, dtype=np.uint8).reshape(600, 800, 4)

# Convert to PIL Image
img = Image.fromarray(arr, mode='RGBA')
img.save('output.png')
```

## License

MIT OR Apache-2.0 (same as skia-rs)
