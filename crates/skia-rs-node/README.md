# @skia-rs/node

Node.js bindings for skia-rs, a 100% Rust implementation of Google's Skia 2D graphics library.

## Installation

### From npm (when published)

```bash
npm install @skia-rs/node
```

### From Source

```bash
# Install dependencies
npm install

# Build native module
npm run build
```

## Quick Start

```javascript
const skia = require('@skia-rs/node');

// Create a surface
const surface = new skia.Surface(800, 600);

// Create a paint
const paint = new skia.Paint();
paint.setColor(skia.RED);
paint.setAntiAlias(true);

// Clear and draw
surface.clear(skia.WHITE);
surface.drawCircle(400, 300, 100, paint);

// Get pixel data
const pixels = surface.getPixels(); // Buffer
```

## API Reference

### Surface

```javascript
const surface = new skia.Surface(width, height);
surface.width       // number: Width in pixels
surface.height      // number: Height in pixels
surface.clear(color)
surface.drawRect(left, top, right, bottom, paint)
surface.drawCircle(cx, cy, radius, paint)
surface.drawOval(left, top, right, bottom, paint)
surface.drawLine(x0, y0, x1, y1, paint)
surface.drawPath(path, paint)
surface.drawPoint(x, y, paint)
surface.getPixels()    // Buffer: RGBA pixel data
surface.getRowBytes()  // number: Bytes per row
```

### Paint

```javascript
const paint = new skia.Paint();
paint.getColor() / paint.setColor(color)      // ARGB color
paint.getStyle() / paint.setStyle(style)      // 0=fill, 1=stroke, 2=both
paint.getStrokeWidth() / paint.setStrokeWidth(width)
paint.getAntiAlias() / paint.setAntiAlias(enabled)
paint.getAlpha() / paint.setAlpha(alpha)      // 0-255
paint.setArgb(a, r, g, b)                      // Set from components
```

### Path & PathBuilder

```javascript
const builder = new skia.PathBuilder();
builder.moveTo(0, 0)
builder.lineTo(100, 0)
builder.quadTo(150, 50, 100, 100)
builder.cubicTo(50, 150, 0, 150, 0, 100)
builder.close()
builder.addRect(10, 10, 50, 50)
builder.addCircle(100, 100, 25)
builder.addOval(0, 0, 100, 50)
builder.addRoundRect(0, 0, 100, 100, 10, 10)
builder.reset()

const path = builder.build();
path.isEmpty()
path.getBounds()     // Rect
path.contains(x, y)  // boolean
```

### Geometry

```javascript
// Point
const p = new skia.Point(10, 20);
p.x, p.y
p.length()
p.normalize()
p.add(other), p.sub(other), p.mul(2.0)

// Rect
const r = new skia.Rect(0, 0, 100, 100);
const r = skia.Rect.fromXywh(10, 10, 80, 80);
const r = skia.Rect.fromWh(100, 100);
r.left, r.top, r.right, r.bottom
r.width, r.height
r.isEmpty()
r.contains(50, 50)
r.center()  // Point

// Matrix
const m = new skia.Matrix();           // Identity
const m = skia.Matrix.translate(10, 20);
const m = skia.Matrix.scale(2, 2);
const m = skia.Matrix.rotate(radians);
const m = skia.Matrix.rotateDeg(45);
m.concat(other)  // Matrix multiplication
m.invert()       // Matrix | null
m.mapPoint(x, y) // Point
m.getValues()    // number[]
```

### Colors

```javascript
// Color utilities
const color = skia.argb(255, 255, 0, 0);  // Red
const color = skia.rgb(0, 255, 0);        // Green (opaque)

// Predefined colors
skia.BLACK
skia.WHITE
skia.RED
skia.GREEN
skia.BLUE
skia.YELLOW
skia.CYAN
skia.MAGENTA
skia.TRANSPARENT
```

## Examples

### Drawing Shapes

```javascript
const skia = require('@skia-rs/node');

const surface = new skia.Surface(400, 400);
surface.clear(skia.WHITE);

// Filled rectangle
const fill = new skia.Paint();
fill.setArgb(255, 100, 150, 200);
surface.drawRect(50, 50, 150, 150, fill);

// Stroked circle
const stroke = new skia.Paint();
stroke.setStyle(1); // stroke
stroke.setStrokeWidth(3);
stroke.setColor(skia.RED);
surface.drawCircle(300, 100, 50, stroke);

// Path
const builder = new skia.PathBuilder();
builder.moveTo(200, 200);
builder.lineTo(300, 250);
builder.lineTo(250, 350);
builder.close();

const pathPaint = new skia.Paint();
pathPaint.setArgb(128, 0, 255, 0); // Semi-transparent green
surface.drawPath(builder.build(), pathPaint);
```

### Saving to PNG

```javascript
const skia = require('@skia-rs/node');
const fs = require('fs');
const { PNG } = require('pngjs');

const surface = new skia.Surface(800, 600);
surface.clear(skia.WHITE);
// ... draw operations ...

// Get pixels and save
const pixels = surface.getPixels();
const png = new PNG({ width: surface.width, height: surface.height });
png.data = pixels;
const buffer = PNG.sync.write(png);
fs.writeFileSync('output.png', buffer);
```

### Using with Canvas/sharp

```javascript
const skia = require('@skia-rs/node');
const sharp = require('sharp');

const surface = new skia.Surface(800, 600);
surface.clear(skia.WHITE);
// ... draw operations ...

// Convert to PNG using sharp
const pixels = surface.getPixels();
await sharp(pixels, {
  raw: {
    width: surface.width,
    height: surface.height,
    channels: 4
  }
}).png().toFile('output.png');
```

## TypeScript

TypeScript definitions are included:

```typescript
import * as skia from '@skia-rs/node';

const surface: skia.Surface = new skia.Surface(800, 600);
const paint: skia.Paint = new skia.Paint();
paint.setColor(0xFFFF0000);
```

## License

MIT OR Apache-2.0 (same as skia-rs)
