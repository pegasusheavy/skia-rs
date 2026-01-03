#!/usr/bin/env node
/**
 * Basic drawing example for skia-rs Node.js bindings.
 *
 * This example demonstrates:
 * - Creating a surface
 * - Drawing basic shapes (rectangles, circles, lines)
 * - Using different paint styles (fill, stroke)
 * - Working with paths
 * - Exporting to PNG (requires pngjs)
 */

const skia = require('../index.js');
const fs = require('fs');

// Optional: for saving to PNG
let PNG;
try {
    PNG = require('pngjs').PNG;
} catch (e) {
    console.log('Note: pngjs not installed. Cannot save PNG.');
    console.log('Run: npm install pngjs');
}

function main() {
    // Create a 800x600 surface
    const width = 800;
    const height = 600;
    const surface = new skia.Surface(width, height);

    // Clear with white background
    surface.clear(skia.WHITE);

    // Create a filled rectangle paint
    const fillPaint = new skia.Paint();
    fillPaint.setArgb(255, 100, 150, 200); // Light blue
    fillPaint.setAntiAlias(true);

    // Draw a filled rectangle
    surface.drawRect(50, 50, 250, 200, fillPaint);

    // Create a stroke paint
    const strokePaint = new skia.Paint();
    strokePaint.setStyle(1); // stroke
    strokePaint.setStrokeWidth(4.0);
    strokePaint.setColor(skia.RED);
    strokePaint.setAntiAlias(true);

    // Draw a stroked circle
    surface.drawCircle(400, 150, 80, strokePaint);

    // Draw an oval
    const ovalPaint = new skia.Paint();
    ovalPaint.setArgb(180, 0, 200, 100); // Semi-transparent green
    surface.drawOval(500, 50, 750, 200, ovalPaint);

    // Draw lines
    const linePaint = new skia.Paint();
    linePaint.setStyle(1); // stroke
    linePaint.setStrokeWidth(2.0);
    linePaint.setColor(skia.BLUE);

    for (let i = 0; i < 10; i++) {
        const y = 250 + i * 10;
        surface.drawLine(50, y, 350, y + 50, linePaint);
    }

    // Build and draw a path (a star shape)
    const builder = new skia.PathBuilder();
    const cx = 600, cy = 400;
    const outerR = 100, innerR = 40;

    for (let i = 0; i < 5; i++) {
        const angleOuter = (i * 72 - 90) * Math.PI / 180;
        const angleInner = (i * 72 + 36 - 90) * Math.PI / 180;

        const ox = cx + outerR * Math.cos(angleOuter);
        const oy = cy + outerR * Math.sin(angleOuter);
        const ix = cx + innerR * Math.cos(angleInner);
        const iy = cy + innerR * Math.sin(angleInner);

        if (i === 0) {
            builder.moveTo(ox, oy);
        } else {
            builder.lineTo(ox, oy);
        }
        builder.lineTo(ix, iy);
    }
    builder.close();

    const starPath = builder.build();

    const starPaint = new skia.Paint();
    starPaint.setColor(skia.YELLOW);
    surface.drawPath(starPath, starPaint);

    // Stroke the star outline
    const starStroke = new skia.Paint();
    starStroke.setStyle(1); // stroke
    starStroke.setStrokeWidth(2.0);
    starStroke.setArgb(255, 200, 150, 0); // Dark yellow
    surface.drawPath(starPath, starStroke);

    // Draw a rounded rectangle
    const roundedPaint = new skia.Paint();
    roundedPaint.setArgb(200, 150, 100, 200); // Purple

    const builder2 = new skia.PathBuilder();
    builder2.addRoundRect(50, 350, 300, 550, 20, 20);
    surface.drawPath(builder2.build(), roundedPaint);

    console.log(`Created ${width}x${height} image`);
    const bounds = starPath.getBounds();
    console.log(`Star path bounds: Rect(${bounds.left}, ${bounds.top}, ${bounds.right}, ${bounds.bottom})`);
    console.log(`Star contains center: ${starPath.contains(600, 400)}`);

    // Save to PNG if pngjs is available
    if (PNG) {
        const pixels = surface.getPixels();
        const png = new PNG({ width, height });
        png.data = pixels;
        const buffer = PNG.sync.write(png);
        fs.writeFileSync('node_drawing.png', buffer);
        console.log('Saved to node_drawing.png');
    }
}

main();
