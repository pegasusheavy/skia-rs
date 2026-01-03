# skia-rs-skottie

Lottie animation support for skia-rs, providing a Rust implementation of Skia's Skottie library.

## Features

- **JSON Parsing**: Full Lottie JSON format support
- **Animation Playback**: Timeline-based animation with frame interpolation
- **Shape Layers**: Paths, fills, strokes, gradients
- **Transform Animations**: Position, scale, rotation, opacity
- **Masks & Mattes**: Alpha masks, track mattes
- **Expressions**: Subset of Lottie expressions

## Usage

```rust
use skia_rs_skottie::{Animation, AnimationBuilder};

// Load from JSON string
let animation = Animation::from_json(json_string)?;

// Get animation properties
println!("Duration: {} seconds", animation.duration());
println!("Frame rate: {} fps", animation.fps());
println!("Size: {}x{}", animation.width(), animation.height());

// Render a specific frame
let canvas = /* your canvas */;
animation.render(canvas, frame_time);

// Or seek to normalized time (0.0 - 1.0)
animation.seek(0.5); // Seek to 50%
animation.render(canvas, None);
```

## Supported Features

### Layers
- Shape layers
- Solid layers
- Image layers (references)
- Precomposition layers
- Null layers

### Shapes
- Rectangle
- Ellipse
- Path
- Polystar
- Fill
- Stroke
- Gradient fill/stroke
- Group
- Trim paths

### Transforms
- Position
- Anchor point
- Scale
- Rotation
- Opacity
- Skew

### Effects
- Masks (Add, Subtract, Intersect, Difference)
- Track mattes (Alpha, Alpha Inverted, Luma, Luma Inverted)

## License

MIT OR Apache-2.0
