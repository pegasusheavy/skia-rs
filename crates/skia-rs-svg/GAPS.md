# skia-rs-svg Gap Analysis

**Date:** 2026-04-25
**Reviewer:** Claude (Opus 4.7)
**Last updated:** 2026-04-26 (P7E closure — N-1 CSS Color Level 4 fully resolved)

## Summary

- Total public functions reviewed: ~75 (`pub fn` across css.rs, dom.rs, export.rs, parser.rs, render.rs)
- Total test functions: **39** (21 original + 18 new, all passing)
  - css.rs: 9 (+2 for apply-to-DOM and inline precedence)
  - dom.rs: 2
  - export.rs: 7 (+2 for gradientTransform + style round-trip)
  - parser.rs: 11 (+7 for text content, entities, stops, style, inline style, image, radial stops)
  - render.rs: 10 (+7 for pixel sampling, CSS, inline style, linear gradient, clip-path, URL, base64, text)
- Total gaps found: 13 — **all resolved**
- Critical gaps: 5 **(all resolved)**
- Nice-to-have gaps: 5 **(all resolved)**
- Test coverage gaps: 3 **(all resolved)**
- Estimated complexity: **Medium-High** — completed in the P6D phase.

## Files Reviewed
- [x] lib.rs (23 lines)
- [x] parser.rs (609 lines)
- [x] dom.rs (394 lines)
- [x] render.rs (261 lines)
- [x] css.rs (764 lines)
- [x] export.rs (850 lines)

## Critical Gaps

### C-1: `render_node` does not render `SvgNodeKind::Text`
**File:** `render.rs` (lines 149-152)
**Severity:** Critical
**Description:** The `Text` arm of the match is a no-op with the comment "Text rendering requires font support // For now, skip text nodes". skia-rs-text exists and provides real text measurement / glyph generation / TextBlob building; the SVG renderer does not pull it in. Every SVG with `<text>` content renders as blank space.
**Impact:** SVG text content — titles, labels, logos, data-vis — is completely invisible. This is a bedrock feature.
**Effort:** Medium (add skia-rs-text dependency; convert `SvgText` to a `TextBlob` via `Font::from_size(svg_text.font_size)` and `font.text_to_glyphs`, draw via `Canvas::draw_text_blob` or a per-glyph path-draw loop; handle `text-anchor` alignment; ~80 lines).
**Status:** RESOLVED — `render_text` in `render.rs` converts each char to a glyph via `skia_rs_text::Font::from_size`, fetches `glyph_path`, translates by the per-glyph advance, and draws fill + stroke. `text-anchor` (start/middle/end) is honored against `measure_text`. Covered by `test_render_text_does_not_panic`.

### C-2: `render_node` does not resolve `SvgPaint::Url(...)` to gradients or patterns
**File:** `render.rs` (lines 182-207)
**Severity:** Critical
**Description:** `create_paint_from_svg_paint` returns a default Paint with no color/shader when it sees a `SvgPaint::Url(_)`. Comment: "Gradient/pattern lookup would go here // For now, return a default paint." So every element with `fill="url(#myGradient)"` or `stroke="url(#pattern)"` draws with default Paint (black by default). The gradient definitions are parsed correctly into `SvgNodeKind::LinearGradient`/`RadialGradient` and sit in the DOM inside `<defs>`, but nothing looks them up. Compare skia-rs-paint where `LinearGradient`/`RadialGradient` shaders are implemented and sampleable; the bridge from SVG gradient AST → Paint shader is missing.
**Impact:** Any SVG using gradients, patterns, or referenced paints renders with flat default-color shapes.
**Effort:** Medium (traverse the DOM once to build `HashMap<String, SvgNodeKind>` of defs; in `create_paint_from_svg_paint`, when we see `Url(id)`, look up the referenced gradient and construct a `skia_rs_paint::LinearGradient` / `RadialGradient` from its stops/transform/units; ~150 lines).
**Status:** RESOLVED — `RenderContext` builds a `HashMap<String, &SvgNode>` once per render pass. `create_paint_from_svg_paint` calls `build_gradient_shader`, which constructs `skia_rs_paint::LinearGradient`/`RadialGradient` from the resolved AST with stops mapped to `Color4f`, respects `gradientTransform` as a local matrix, maps spreadMethod to `TileMode`, and handles both `objectBoundingBox` and `userSpaceOnUse` units. Covered by `test_render_linear_gradient` (asserts left=red, right=blue pixels after gradient).

### C-3: `SvgLinearGradient`/`SvgRadialGradient` `stops` arrays are never populated
**File:** `parser.rs` (lines 252-298)
**Severity:** Critical
**Description:** The parser creates a `SvgLinearGradient { ..., stops: Vec::new(), ... }` or `SvgRadialGradient { ..., stops: Vec::new(), ... }` with an empty stops vector. Nothing in `parse_svg` ever reads `<stop>` children and appends to the gradient's stops. The `stops` field is therefore always empty even for well-formed `<linearGradient><stop offset="0" stop-color="red"/>...</linearGradient>`. Cannot blame the Url lookup (C-2) for blank gradients — even once C-2 is fixed, there are no stops to interpolate.
**Impact:** SVG gradient parsing is a no-op below the gradient-element level. The AST carries an empty `stops: []` regardless of source.
**Effort:** Small (during the child-add flow in `parse_svg`, detect `stop` elements parented under a gradient and append to its `stops`; also handle `stop-color` and `stop-opacity` attributes; ~40 lines).
**Status:** RESOLVED — `collect_stops` walks the gradient's `<stop>` children and populates `GradientStop { offset, color, opacity }`. `stop-color` / `stop-opacity` can come from attributes or the `style` attribute (via `resolve_stop_style`). Stops are also skipped from the rendered tree so they don't appear as Unknown nodes. Covered by `test_gradient_stops_parsed` and `test_radial_gradient_stops`.

### C-4: No CSS selector / stylesheet application in render path
**File:** `css.rs` (764 lines), `render.rs`
**Severity:** Critical
**Description:** `Stylesheet::parse` parses CSS rules into `CssRule` AST and `apply_stylesheet` is exposed, but `render_svg`/`render_svg_string` never calls it. Inline `style="fill:red"` attributes are also not parsed — the parser reads `fill`, `stroke`, etc. as separate attributes but ignores `style` entirely. Only `parse_inline_style` is exposed for the caller to handle manually. This means CSS-styled SVGs (including most modern exports from Figma/Illustrator/Inkscape which use classes heavily) render without any CSS-applied styling.
**Impact:** Real-world SVG content from design tools fails to render with intended colors/strokes/opacities. Users must write a `<svg>` with inline attributes on every element to get any styling.
**Effort:** Medium (1) make `parse_svg` call `Stylesheet::parse` on `<style>` element content, storing the stylesheet on the DOM; (2) in `render_svg` or a pre-render walk, call `apply_stylesheet` on each node; (3) parse each node's `style=` attribute via `parse_inline_style` during `create_node`; ~100 lines).
**Status:** RESOLVED — `parse_svg` calls `Stylesheet::parse` on every `<style>` element's text content and stores the merged result on `SvgDom.stylesheet`. Inline `style="..."` is preserved on `SvgNode.attributes["style"]` so `apply_stylesheet` picks it up in the inline-override pass. `render_svg` clones the DOM, applies the stylesheet, then walks for rendering (source DOM is untouched). Covered by `test_apply_stylesheet_to_dom`, `test_apply_stylesheet_inline_overrides_rule`, `test_render_css_styled`, `test_render_inline_style`.

### C-5: `SvgNodeKind::Image` is rendered as a no-op; `ClipPath` and `Defs` dispatch paths are wrong/missing
**File:** `render.rs` (lines 168-170, 160-167)
**Severity:** Critical
**Description:** (a) `SvgNodeKind::Image(_img)` arm is `// Image rendering requires image loading support` with no body — every `<image>` element is silently dropped. Data-URI `href`s and external URLs both need decoding (the `href` lives in `img.href`); a real impl would base64-decode the data URI into `skia_rs_codec::decode_image`, build a Paint with an ImageShader, and `canvas.draw_image`. (b) `Defs` falls into the catch-all `SvgNodeKind::Group | SvgNodeKind::Svg | SvgNodeKind::Defs` arm which renders children unless the node is `Defs` — which is correct except this only affects `<defs>` at the top; gradient nodes that happen to be direct children of `<svg>` (common shortcut) get rendered as empty groups. (c) `ClipPath(String)` is parsed into the DOM but the render match never handles it — the variant is unreachable in the current arms and falls through to `_ => { render children }`, so clip-path IDs become renderable groups that draw their contents.
**Impact:** Images and clipped content rendering is broken. Clip-paths fail closed (content ignored); images fail open (no output). This is a visual-bug minefield.
**Effort:** Medium-High (image support ~80 lines; clipPath requires integrating `canvas.clip_path` with the referenced clip contents; ~100 lines each).
**Status:** RESOLVED — (a) `render_image` decodes `data:...;base64,` hrefs via a local `decode_base64` (no new dep) and passes the payload to `skia_rs_codec::decode_image`, then calls `Canvas::draw_image_rect` into the element's x/y/w/h box; external URLs are a documented no-op since the crate does no network I/O. (b) `Defs`, `LinearGradient`, `RadialGradient`, and `ClipPath` now have their own match arms that explicitly do nothing (definition-only elements). (c) `clip-path="url(#id)"` on any node looks up the referenced `ClipPath` via the `RenderContext` and calls `canvas.clip_path(&built, ClipOp::Intersect, true)`. Covered by `test_render_clip_path` (pixel-level assertion that inside-clip is red, outside-clip is background), and by the `decode_base64` + href parser unit tests.

## Nice-to-Have Gaps

### N-1: `parse_color` lacks HSL/HSLA/LCH/OKLCH and modern CSS color syntax
**File:** `parser.rs` (lines 409-473)
**Severity:** Nice-to-have
**Description:** Supports `#rgb`, `#rrggbb`, `rgb()`, `rgba()`, and ~15 named colors. Missing: 3-digit hex alpha (`#rgba`), 8-digit hex (`#rrggbbaa`), `hsl()`/`hsla()`, `hwb()`, `lab()`, `lch()`, `oklab()`, `oklch()`, `color()`, and the other ~130 named colors defined in CSS Color Level 4 (aliceblue, cornflowerblue, etc.).
**Impact:** Many SVG files from modern tools use `hsl()` or named colors outside the 15-color list.
**Effort:** Small-Medium (mostly mechanical; use a lookup table for the full named-color list).
**Status:** ✅ FULLY RESOLVED (P7E) — `parse_color` now handles 3/4/6/8-digit hex, `rgb()`, `rgba()`, `hsl()`, `hsla()`, and the full CSS Color 3 named-color table (~140 colors). CSS Color Level 4 modern color syntaxes (`lab()`, `lch()`, `oklab()`, `oklch()`, `hwb()`, `color(srgb|srgb-linear|display-p3|a98-rgb ...)`) are now supported via `Color::from_css` in skia-rs-core (P7E). Values are converted to sRGB; out-of-gamut colors are clamped. SVG's `parse_color` delegates to `Color::from_css`, so all Level 4 syntax is available. Covered by extended `test_parse_color` assertions in SVG and the comprehensive Level 4 test suite in skia-rs-core.

### N-2: `parse_length` does not handle `rem`/`vw`/`vh`/`ch`/`ex`/negative zero
**File:** `parser.rs` (lines 343-357)
**Severity:** Nice-to-have
**Description:** Handles `%`, `px`, `pt`, `em`, and plain numbers. The `em` conversion hardcodes `16.0` instead of the element's font size. Missing: `rem`, `vw`, `vh`, `ch`, `ex`, `vmin`, `vmax`, `cm`, `mm`, `in`. The `%` conversion returns a fraction in the range [0,1] with no context about what it's a percentage of (width / height / viewport / parent).
**Impact:** Responsive SVGs using relative units render with wrong sizes.
**Effort:** Medium (need a viewport/context struct threaded through the renderer).
**Status:** RESOLVED — `parse_length` now handles `px`, `pt`, `pc`, `em`, `rem`, `ex`, `ch`, `vw`, `vh`, `vmin`, `vmax`, `cm`, `mm`, `in`, plus `%` and unit-less. Physical units convert at 96dpi. Viewport units resolve against a 1000-unit-square default (documented tradeoff — threading the real viewport into the parser would require rewriting every call site; the renderer applies the actual viewBox transform separately). Covered by expanded `test_parse_length` (`1in`, `1cm`, `1em`, `1rem`).

### N-3: `parse_svg` uses a hand-rolled character-by-character XML parser
**File:** `parser.rs` (lines 24-162)
**Severity:** Nice-to-have
**Description:** Comment acknowledges: "Simple state-machine parser for basic SVG. A full implementation would use roxmltree." The hand-rolled parser: (a) does not handle XML namespaces (`xmlns:xlink` is stripped via `or_else(|| attrs.get("xlink:href"))` as a one-off in `use`; other namespaced attrs are lost); (b) does not decode entities (`&amp;`, `&#39;`, `&#x20;`); (c) does not parse CDATA sections; (d) does not collect text content between open/close tags into the `SvgText::content` field (line 236 `content: String::new(), // Will be filled with text content` is a lie — nothing ever fills it); (e) does not handle comments inside tags or XML-style empty tags `<br/>` except by a simple single-character check.
**Impact:** Any SVG with text content (empty strings in `content`), entities (raw `&`), namespaces, or CDATA breaks. This compounds C-1 because even a fixed text renderer would have no content to render.
**Effort:** Medium (switch to roxmltree which is already in the skia-rs dep tree via other crates; rewrite `parse_svg` to walk the roxmltree DOM; ~250 lines but eliminates all hand-parsing bugs).
**Status:** RESOLVED — `parse_svg` now uses `roxmltree::Document::parse`. Entities, CDATA, comments, and namespaced attributes (stored under both local name and `prefix:local` form so `xlink:href` still works) are all handled by the library. Covered by `test_xml_entities_decoded`.

### N-4: `SvgText::content` is never filled from XML text nodes
**File:** `parser.rs` (lines 232-250)
**Severity:** Nice-to-have (but see N-3 above — same root cause)
**Description:** `<text x="10" y="20">Hello, world!</text>` creates an `SvgText { content: String::new(), ... }` — the "Hello, world!" text is lost. The hand-rolled parser never accumulates text between `<text>` and `</text>`.
**Impact:** Text rendering (C-1) is doubly-broken: the content is empty even before reaching the renderer.
**Effort:** Small if switching to roxmltree (N-3); medium if patching the hand-rolled parser.
**Status:** RESOLVED — `collect_text` walks all descendant text nodes of `<text>` and concatenates them into `SvgText::content`. Covered by `test_text_content_captured`.

### N-5: Export: only some node kinds round-trip
**File:** `export.rs` (850 lines — large, reviewed selectively)
**Severity:** Nice-to-have
**Description:** `export_svg` walks nodes and emits XML for the common shape kinds. Rarely-used kinds (Polyline/Polygon/Use/LinearGradient/RadialGradient/ClipPath/Image/Text) have export code but with limited attribute coverage. Round-trip SVG → DOM → SVG loses ordering of `<defs>` children, gradient transform matrices if they're non-identity (export hardcodes identity transform for gradients), and CSS `<style>` elements (which the current parser ignores entirely, so there's nothing to export).
**Effort:** Medium.
**Status:** RESOLVED — `export_gradient_transform_attr` emits `gradientTransform=` (not `transform=`) per SVG 1.1 §13.2.3. The `<style>` Unknown-branch re-emits the preserved `__text_content` inside a `<![CDATA[...]]>` block so SVG → DOM → SVG → DOM round-trips preserve the rule set. Covered by `test_export_gradient_transform_uses_gradient_attribute` and `test_export_style_element_round_trips`.

## Test Coverage Gaps

### T-1: Rendering tests only assert `surface.is_some()` — no pixel-level verification
**Description:** `test_render_simple_svg`, `test_render_circle`, `test_render_path` build a surface and assert it exists with the expected dimensions. They do not sample the surface to check that pixels were actually drawn in the expected color / at the expected position. All three rendering bugs in C-1 through C-5 would pass this test.
**Effort:** Small (`surface.peek_pixels()` + spot-check at known points).
**Status:** RESOLVED — `pixel_at` helper samples RGBA8 pixels from `Surface::pixels()`. Every original render test plus the new `test_render_css_styled`, `test_render_inline_style`, `test_render_linear_gradient`, and `test_render_clip_path` assert concrete expected RGB values at known coordinates.

### T-2: Gradient parsing has zero tests
**Description:** Neither `parse_svg` tests for `<linearGradient>`/`<radialGradient>` nor any test covers stop extraction. The C-3 bug (empty stops) is invisible to the test suite.
**Effort:** Small.
**Status:** RESOLVED — `test_gradient_stops_parsed` covers `<linearGradient>` with three stops (attribute-colour, attribute-colour + opacity, style-attribute colour) and asserts the resulting `GradientStop` offsets/colors/opacities. `test_radial_gradient_stops` covers the radial variant.

### T-3: CSS tests cover parser only, not application to a DOM
**Description:** The 14 tests in `css.rs` exercise `Stylesheet::parse` and `CssSelector` matching in isolation, but no test calls `apply_stylesheet(stylesheet, &mut dom)` and verifies that the DOM's nodes pick up the styled attributes. C-4 (stylesheet not applied during render) is therefore untested end-to-end.
**Effort:** Small.
**Status:** RESOLVED — `test_apply_stylesheet_to_dom` builds a DOM, applies a stylesheet with a class selector, and asserts `fill`/`opacity` land on the targeted node. `test_apply_stylesheet_inline_overrides_rule` verifies the inline `style="..."` beats a matching `#id { ... }` rule (since inline is applied last). End-to-end render coverage is provided separately by `test_render_css_styled` and `test_render_inline_style`.

## Implementation Notes

### Hand-rolled XML parsing vs. roxmltree
The crate includes roxmltree-shaped code (state machine, self-closing tag detection, attribute-quote handling) but does not depend on roxmltree. `roxmltree` is a well-maintained parser with proper namespace, entity, and CDATA handling. Switching would eliminate ~200 lines of fragile hand-parser code and a class of subtle bugs (N-3, N-4).

### Cross-crate integration is the core problem
Most of the critical gaps (C-1 text, C-2 gradient URL lookup, C-5 image) are wiring issues between skia-rs-svg and sister crates (skia-rs-text, skia-rs-paint shaders, skia-rs-codec image decode). Each gap is individually small; the aggregate says nobody has connected the SVG renderer to the rest of the library yet.

### CSS work is surprisingly mature
`css.rs` is 764 lines with 14 tests and contains a real selector matcher (element, class, ID, descendant, multiple) with specificity calculation. The problem is only that `render_svg` doesn't call `apply_stylesheet` — all the machinery is built.

### SVG path parsing is delegated to skia-rs-path
`parser.rs` line 229: `let path = parse_svg_path(d).unwrap_or_default();` — this calls into skia-rs-path's SVG path mini-language parser. That parser is well-tested (per earlier Phase 3 audit in skia-rs-path/GAPS.md) and correctly handles M/L/H/V/C/S/Q/T/A commands with relative variants. Cross-crate wiring here is correct.

### `Matrix` integration is correct
`parse_transform_str` handles `translate/scale/rotate/skewX/skewY/matrix` and produces a proper `skia_rs_core::Matrix`. The only quirk is skewX/skewY using `angle.to_radians().tan()` instead of the angle directly — which is actually correct, because CSS transform-skew takes an angle and applies tan to it (matches SVG spec).

## Recommendations

### Priority 1: Wire SVG → skia-rs-text for `<text>` rendering (C-1, N-4 fallout)
This is the single biggest user-visible gap. Combine with N-3/roxmltree switch so that text content is actually captured from the XML. Estimated effort: 2-3 days.

### Priority 2: Populate gradient `stops` and resolve `Url(...)` in paints (C-2, C-3)
Small fixes, large visual impact. Do both in one PR — neither is useful without the other. ~1 day.

### Priority 3: Apply CSS stylesheets during render (C-4)
The CSS parser is already strong; just wire it into the render pipeline and handle `style=` attributes on nodes. ~1 day.

### Priority 4: Replace hand-rolled XML parser with roxmltree (N-3, N-4)
Cleanup pass that unlocks N-4 as a side effect and eliminates an entire class of edge-case bugs (entities, namespaces, CDATA). Do this before piling more features onto the current parser. ~2 days.

### Priority 5: Image and clipPath rendering (C-5)
`<image>` via skia-rs-codec; clipPath via `canvas.clip_path`. Each ~0.5-1 day.

### Priority 6: CSS color extensions, test coverage, gradient export (N-1, T-1/T-2/T-3, N-5)
Polish work. 2-3 days total.
