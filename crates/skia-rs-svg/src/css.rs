//! CSS parsing and styling support for SVG.
//!
//! This module provides CSS parsing and style application for SVG documents,
//! including support for:
//! - Inline styles (`style` attribute)
//! - Embedded stylesheets (`<style>` elements)
//! - Selectors: element, class, ID, descendant, multiple
//! - Cascading and specificity

use crate::dom::{SvgDom, SvgNode, SvgPaint};
use skia_rs_core::{Color, Matrix, Scalar};
use std::collections::HashMap;

/// A CSS stylesheet containing multiple rules.
#[derive(Debug, Clone, Default)]
pub struct Stylesheet {
    /// CSS rules in order.
    pub rules: Vec<CssRule>,
}

impl Stylesheet {
    /// Create an empty stylesheet.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Parse a CSS stylesheet from a string.
    pub fn parse(css: &str) -> Self {
        let mut stylesheet = Self::new();
        let css = css.trim();

        // Simple CSS parser
        let mut chars = css.chars().peekable();

        while chars.peek().is_some() {
            // Skip whitespace
            while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
                chars.next();
            }

            if chars.peek().is_none() {
                break;
            }

            // Skip comments
            if chars.peek() == Some(&'/') {
                chars.next();
                if chars.peek() == Some(&'*') {
                    chars.next();
                    // Skip until */
                    loop {
                        match chars.next() {
                            Some('*') if chars.peek() == Some(&'/') => {
                                chars.next();
                                break;
                            }
                            None => break,
                            _ => {}
                        }
                    }
                    continue;
                }
            }

            // Read selector until {
            let mut selector = String::new();
            while let Some(&c) = chars.peek() {
                if c == '{' {
                    chars.next();
                    break;
                }
                selector.push(chars.next().unwrap());
            }

            let selector = selector.trim().to_string();
            if selector.is_empty() {
                break;
            }

            // Read declarations until }
            let mut declarations_str = String::new();
            let mut brace_depth = 1;
            while let Some(c) = chars.next() {
                if c == '{' {
                    brace_depth += 1;
                } else if c == '}' {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        break;
                    }
                }
                declarations_str.push(c);
            }

            // Parse declarations
            let declarations = parse_declarations(&declarations_str);

            // Parse selector into multiple selectors (comma-separated)
            for sel in selector.split(',') {
                let sel = sel.trim();
                if !sel.is_empty() {
                    stylesheet.rules.push(CssRule {
                        selector: CssSelector::parse(sel),
                        declarations: declarations.clone(),
                    });
                }
            }
        }

        stylesheet
    }

    /// Add a rule to the stylesheet.
    pub fn add_rule(&mut self, rule: CssRule) {
        self.rules.push(rule);
    }
}

/// A CSS rule with selector and declarations.
#[derive(Debug, Clone)]
pub struct CssRule {
    /// The selector for this rule.
    pub selector: CssSelector,
    /// Property declarations.
    pub declarations: StyleDeclarations,
}

/// A CSS selector.
#[derive(Debug, Clone)]
pub enum CssSelector {
    /// Universal selector (*).
    Universal,
    /// Element selector (e.g., rect, circle).
    Element(String),
    /// Class selector (e.g., .classname).
    Class(String),
    /// ID selector (e.g., #id).
    Id(String),
    /// Descendant selector (e.g., g rect).
    Descendant(Box<CssSelector>, Box<CssSelector>),
    /// Child selector (e.g., g > rect).
    Child(Box<CssSelector>, Box<CssSelector>),
    /// Multiple conditions (e.g., rect.classname).
    And(Vec<CssSelector>),
}

impl CssSelector {
    /// Parse a selector string.
    pub fn parse(s: &str) -> Self {
        let s = s.trim();

        // Check for descendant/child selectors
        if s.contains(" > ") {
            let parts: Vec<&str> = s.splitn(2, " > ").collect();
            if parts.len() == 2 {
                return CssSelector::Child(
                    Box::new(Self::parse(parts[0])),
                    Box::new(Self::parse(parts[1])),
                );
            }
        }

        if s.contains(' ') {
            let parts: Vec<&str> = s.splitn(2, ' ').collect();
            if parts.len() == 2 && !parts[1].is_empty() {
                return CssSelector::Descendant(
                    Box::new(Self::parse(parts[0])),
                    Box::new(Self::parse(parts[1])),
                );
            }
        }

        // Check for combined selectors (e.g., rect.classname#id)
        let mut selectors = Vec::new();
        let mut current = String::new();
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '.' | '#' => {
                    if !current.is_empty() {
                        selectors.push(parse_simple_selector(&current));
                        current.clear();
                    }
                    current.push(c);
                }
                _ => current.push(c),
            }
        }

        if !current.is_empty() {
            selectors.push(parse_simple_selector(&current));
        }

        if selectors.len() == 1 {
            selectors.pop().unwrap()
        } else if selectors.is_empty() {
            CssSelector::Universal
        } else {
            CssSelector::And(selectors)
        }
    }

    /// Calculate specificity (ID, class, element counts).
    pub fn specificity(&self) -> (u32, u32, u32) {
        match self {
            CssSelector::Universal => (0, 0, 0),
            CssSelector::Element(_) => (0, 0, 1),
            CssSelector::Class(_) => (0, 1, 0),
            CssSelector::Id(_) => (1, 0, 0),
            CssSelector::Descendant(a, b) | CssSelector::Child(a, b) => {
                let (id_a, class_a, elem_a) = a.specificity();
                let (id_b, class_b, elem_b) = b.specificity();
                (id_a + id_b, class_a + class_b, elem_a + elem_b)
            }
            CssSelector::And(selectors) => {
                let mut id = 0;
                let mut class = 0;
                let mut elem = 0;
                for sel in selectors {
                    let (i, c, e) = sel.specificity();
                    id += i;
                    class += c;
                    elem += e;
                }
                (id, class, elem)
            }
        }
    }

    /// Check if this selector matches a node.
    pub fn matches(&self, node: &SvgNode, ancestors: &[&SvgNode]) -> bool {
        match self {
            CssSelector::Universal => true,
            CssSelector::Element(tag) => node_tag_name(node) == tag,
            CssSelector::Class(class) => node.classes.contains(class),
            CssSelector::Id(id) => node.id.as_deref() == Some(id.as_str()),
            CssSelector::Descendant(ancestor_sel, child_sel) => {
                if !child_sel.matches(node, ancestors) {
                    return false;
                }
                // Check if any ancestor matches
                for ancestor in ancestors {
                    if ancestor_sel.matches(ancestor, &[]) {
                        return true;
                    }
                }
                false
            }
            CssSelector::Child(parent_sel, child_sel) => {
                if !child_sel.matches(node, ancestors) {
                    return false;
                }
                // Check immediate parent
                if let Some(parent) = ancestors.last() {
                    parent_sel.matches(parent, &ancestors[..ancestors.len().saturating_sub(1)])
                } else {
                    false
                }
            }
            CssSelector::And(selectors) => selectors.iter().all(|s| s.matches(node, ancestors)),
        }
    }
}

fn parse_simple_selector(s: &str) -> CssSelector {
    if s == "*" {
        CssSelector::Universal
    } else if let Some(class) = s.strip_prefix('.') {
        CssSelector::Class(class.to_string())
    } else if let Some(id) = s.strip_prefix('#') {
        CssSelector::Id(id.to_string())
    } else {
        CssSelector::Element(s.to_string())
    }
}

fn node_tag_name(node: &SvgNode) -> &str {
    use crate::dom::SvgNodeKind;
    match &node.kind {
        SvgNodeKind::Svg => "svg",
        SvgNodeKind::Group => "g",
        SvgNodeKind::Rect(_) => "rect",
        SvgNodeKind::Circle(_) => "circle",
        SvgNodeKind::Ellipse(_) => "ellipse",
        SvgNodeKind::Line(_) => "line",
        SvgNodeKind::Polyline(_) => "polyline",
        SvgNodeKind::Polygon(_) => "polygon",
        SvgNodeKind::Path(_) => "path",
        SvgNodeKind::Text(_) => "text",
        SvgNodeKind::Image(_) => "image",
        SvgNodeKind::Use(_) => "use",
        SvgNodeKind::Defs => "defs",
        SvgNodeKind::LinearGradient(_) => "linearGradient",
        SvgNodeKind::RadialGradient(_) => "radialGradient",
        SvgNodeKind::ClipPath(_) => "clipPath",
        SvgNodeKind::Unknown(name) => name,
    }
}

/// Style declarations (property-value pairs).
pub type StyleDeclarations = HashMap<String, String>;

/// Parse CSS declarations from a string.
pub fn parse_declarations(s: &str) -> StyleDeclarations {
    let mut declarations = HashMap::new();

    for decl in s.split(';') {
        let decl = decl.trim();
        if decl.is_empty() {
            continue;
        }

        if let Some((property, value)) = decl.split_once(':') {
            let property = property.trim().to_string();
            let value = value.trim().to_string();
            declarations.insert(property, value);
        }
    }

    declarations
}

/// Parse an inline style attribute.
pub fn parse_inline_style(style: &str) -> StyleDeclarations {
    parse_declarations(style)
}

/// Apply a stylesheet to an SVG DOM.
pub fn apply_stylesheet(dom: &mut SvgDom, stylesheet: &Stylesheet) {
    apply_stylesheet_to_node(&mut dom.root, stylesheet, &[]);
}

fn apply_stylesheet_to_node(node: &mut SvgNode, stylesheet: &Stylesheet, ancestors: &[&SvgNode]) {
    // Collect matching rules with their specificity
    let mut matching_rules: Vec<(&CssRule, (u32, u32, u32))> = stylesheet
        .rules
        .iter()
        .filter(|rule| rule.selector.matches(node, ancestors))
        .map(|rule| (rule, rule.selector.specificity()))
        .collect();

    // Sort by specificity (lower first, so later rules override)
    matching_rules.sort_by_key(|(_, spec)| *spec);

    // Apply declarations in order
    for (rule, _) in matching_rules {
        apply_declarations_to_node(node, &rule.declarations);
    }

    // Also parse and apply inline style attribute if present
    if let Some(style) = node.attributes.get("style").cloned() {
        let inline_decls = parse_inline_style(&style);
        apply_declarations_to_node(node, &inline_decls);
    }

    // Recursively apply to children
    // We need to create a new ancestors list that includes this node
    let node_ptr = node as *const SvgNode;
    let mut new_ancestors: Vec<&SvgNode> = ancestors.to_vec();
    // Safety: we're not modifying ancestors while iterating
    new_ancestors.push(unsafe { &*node_ptr });

    for child in &mut node.children {
        apply_stylesheet_to_node(child, stylesheet, &new_ancestors);
    }
}

fn apply_declarations_to_node(node: &mut SvgNode, declarations: &StyleDeclarations) {
    for (property, value) in declarations {
        apply_style_property(node, property, value);
    }
}

/// Apply a single style property to a node.
pub fn apply_style_property(node: &mut SvgNode, property: &str, value: &str) {
    match property {
        "fill" => {
            node.fill = parse_css_paint(value);
        }
        "stroke" => {
            node.stroke = parse_css_paint(value);
        }
        "stroke-width" => {
            node.stroke_width = parse_css_length(value);
        }
        "opacity" => {
            node.opacity = value.parse().unwrap_or(1.0);
        }
        "fill-opacity" => {
            // Adjust fill opacity
            if let Some(SvgPaint::Color(ref mut color)) = node.fill {
                let opacity: f32 = value.parse().unwrap_or(1.0);
                *color = Color::from_argb(
                    (opacity * 255.0) as u8,
                    color.red(),
                    color.green(),
                    color.blue(),
                );
            }
        }
        "stroke-opacity" => {
            // Adjust stroke opacity
            if let Some(SvgPaint::Color(ref mut color)) = node.stroke {
                let opacity: f32 = value.parse().unwrap_or(1.0);
                *color = Color::from_argb(
                    (opacity * 255.0) as u8,
                    color.red(),
                    color.green(),
                    color.blue(),
                );
            }
        }
        "visibility" => {
            node.visible = value != "hidden";
        }
        "display" => {
            if value == "none" {
                node.visible = false;
            }
        }
        "transform" => {
            node.transform = parse_css_transform(value);
        }
        "font-family" => {
            if let crate::dom::SvgNodeKind::Text(ref mut text) = node.kind {
                text.font_family = Some(value.trim_matches('"').trim_matches('\'').to_string());
            }
        }
        "font-size" => {
            if let crate::dom::SvgNodeKind::Text(ref mut text) = node.kind {
                text.font_size = parse_css_length(value);
            }
        }
        "font-weight" => {
            if let crate::dom::SvgNodeKind::Text(ref mut text) = node.kind {
                text.font_weight = match value {
                    "normal" => 400,
                    "bold" => 700,
                    "lighter" => 300,
                    "bolder" => 800,
                    _ => value.parse().unwrap_or(400),
                };
            }
        }
        "text-anchor" => {
            if let crate::dom::SvgNodeKind::Text(ref mut text) = node.kind {
                text.text_anchor = match value {
                    "middle" => crate::dom::TextAnchor::Middle,
                    "end" => crate::dom::TextAnchor::End,
                    _ => crate::dom::TextAnchor::Start,
                };
            }
        }
        "stroke-linecap" | "stroke-linejoin" | "stroke-dasharray" | "stroke-dashoffset" => {
            // Store in attributes for later use
            node.attributes.insert(property.to_string(), value.to_string());
        }
        _ => {
            // Store unknown properties in attributes
            node.attributes.insert(property.to_string(), value.to_string());
        }
    }
}

fn parse_css_paint(s: &str) -> Option<SvgPaint> {
    let s = s.trim();
    if s == "none" || s == "transparent" {
        Some(SvgPaint::None)
    } else if s.starts_with("url(") {
        let url = s[4..]
            .trim_end_matches(')')
            .trim_matches('"')
            .trim_matches('\'');
        Some(SvgPaint::Url(url.to_string()))
    } else {
        parse_css_color(s).map(SvgPaint::Color)
    }
}

fn parse_css_color(s: &str) -> Option<Color> {
    let s = s.trim();

    if s.starts_with('#') {
        let hex = &s[1..];
        let (r, g, b, a) = if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            (r, g, b, 255)
        } else if hex.len() == 4 {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            let a = u8::from_str_radix(&hex[3..4], 16).ok()? * 17;
            (r, g, b, a)
        } else if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b, 255)
        } else if hex.len() == 8 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            (r, g, b, a)
        } else {
            return None;
        };
        Some(Color::from_argb(a, r, g, b))
    } else if s.starts_with("rgb(") {
        let inner = s[4..].trim_end_matches(')');
        let parts: Vec<u8> = inner
            .split(',')
            .filter_map(|p| p.trim().parse().ok())
            .collect();
        if parts.len() == 3 {
            Some(Color::from_rgb(parts[0], parts[1], parts[2]))
        } else {
            None
        }
    } else if s.starts_with("rgba(") {
        let inner = s[5..].trim_end_matches(')');
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() == 4 {
            let r: u8 = parts[0].parse().ok()?;
            let g: u8 = parts[1].parse().ok()?;
            let b: u8 = parts[2].parse().ok()?;
            let a: f32 = parts[3].parse().ok()?;
            Some(Color::from_argb((a * 255.0) as u8, r, g, b))
        } else {
            None
        }
    } else if s.starts_with("hsl(") || s.starts_with("hsla(") {
        // Parse HSL(A) colors
        let is_hsla = s.starts_with("hsla(");
        let inner = if is_hsla {
            s[5..].trim_end_matches(')')
        } else {
            s[4..].trim_end_matches(')')
        };
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() >= 3 {
            let h: f32 = parts[0].trim_end_matches("deg").parse().ok()?;
            let s_val: f32 = parts[1].trim_end_matches('%').parse::<f32>().ok()? / 100.0;
            let l: f32 = parts[2].trim_end_matches('%').parse::<f32>().ok()? / 100.0;
            let a: f32 = if is_hsla && parts.len() >= 4 {
                parts[3].parse().ok()?
            } else {
                1.0
            };

            let (r, g, b) = hsl_to_rgb(h, s_val, l);
            Some(Color::from_argb((a * 255.0) as u8, r, g, b))
        } else {
            None
        }
    } else {
        // Named colors (extended set)
        match s.to_lowercase().as_str() {
            "black" => Some(Color::BLACK),
            "white" => Some(Color::WHITE),
            "red" => Some(Color::from_rgb(255, 0, 0)),
            "green" => Some(Color::from_rgb(0, 128, 0)),
            "blue" => Some(Color::from_rgb(0, 0, 255)),
            "yellow" => Some(Color::from_rgb(255, 255, 0)),
            "cyan" | "aqua" => Some(Color::from_rgb(0, 255, 255)),
            "magenta" | "fuchsia" => Some(Color::from_rgb(255, 0, 255)),
            "gray" | "grey" => Some(Color::from_rgb(128, 128, 128)),
            "silver" => Some(Color::from_rgb(192, 192, 192)),
            "maroon" => Some(Color::from_rgb(128, 0, 0)),
            "olive" => Some(Color::from_rgb(128, 128, 0)),
            "lime" => Some(Color::from_rgb(0, 255, 0)),
            "teal" => Some(Color::from_rgb(0, 128, 128)),
            "navy" => Some(Color::from_rgb(0, 0, 128)),
            "purple" => Some(Color::from_rgb(128, 0, 128)),
            "orange" => Some(Color::from_rgb(255, 165, 0)),
            "pink" => Some(Color::from_rgb(255, 192, 203)),
            "brown" => Some(Color::from_rgb(165, 42, 42)),
            "coral" => Some(Color::from_rgb(255, 127, 80)),
            "crimson" => Some(Color::from_rgb(220, 20, 60)),
            "gold" => Some(Color::from_rgb(255, 215, 0)),
            "indigo" => Some(Color::from_rgb(75, 0, 130)),
            "ivory" => Some(Color::from_rgb(255, 255, 240)),
            "khaki" => Some(Color::from_rgb(240, 230, 140)),
            "lavender" => Some(Color::from_rgb(230, 230, 250)),
            "lightblue" => Some(Color::from_rgb(173, 216, 230)),
            "lightgreen" => Some(Color::from_rgb(144, 238, 144)),
            "lightgray" | "lightgrey" => Some(Color::from_rgb(211, 211, 211)),
            "darkblue" => Some(Color::from_rgb(0, 0, 139)),
            "darkgreen" => Some(Color::from_rgb(0, 100, 0)),
            "darkred" => Some(Color::from_rgb(139, 0, 0)),
            "darkgray" | "darkgrey" => Some(Color::from_rgb(169, 169, 169)),
            "transparent" => Some(Color::TRANSPARENT),
            "currentcolor" => None, // Would need context
            _ => None,
        }
    }
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let h = h % 360.0 / 360.0;
    
    if s == 0.0 {
        let v = (l * 255.0) as u8;
        return (v, v, v);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

fn parse_css_length(s: &str) -> Scalar {
    let s = s.trim();
    if s.ends_with("px") {
        s[..s.len() - 2].parse().unwrap_or(0.0)
    } else if s.ends_with("pt") {
        s[..s.len() - 2].parse::<Scalar>().unwrap_or(0.0) * 1.333
    } else if s.ends_with("em") {
        s[..s.len() - 2].parse::<Scalar>().unwrap_or(0.0) * 16.0
    } else if s.ends_with("rem") {
        s[..s.len() - 3].parse::<Scalar>().unwrap_or(0.0) * 16.0
    } else if s.ends_with('%') {
        // Percentage - context dependent, return as fraction
        s[..s.len() - 1].parse::<Scalar>().unwrap_or(0.0) / 100.0
    } else {
        s.parse().unwrap_or(0.0)
    }
}

fn parse_css_transform(s: &str) -> Matrix {
    // Reuse the transform parser from the parser module
    crate::parser::parse_transform_str(s)
}

/// Extract embedded stylesheets from an SVG DOM.
pub fn extract_stylesheets(dom: &SvgDom) -> Stylesheet {
    let mut stylesheet = Stylesheet::new();
    extract_stylesheets_from_node(&dom.root, &mut stylesheet);
    stylesheet
}

fn extract_stylesheets_from_node(node: &SvgNode, stylesheet: &mut Stylesheet) {
    // Check for style element content
    if let crate::dom::SvgNodeKind::Unknown(tag) = &node.kind {
        if tag == "style" {
            if let Some(content) = node.attributes.get("__text_content") {
                let parsed = Stylesheet::parse(content);
                stylesheet.rules.extend(parsed.rules);
            }
        }
    }

    for child in &node.children {
        extract_stylesheets_from_node(child, stylesheet);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_css_color() {
        assert_eq!(parse_css_color("#ff0000"), Some(Color::from_rgb(255, 0, 0)));
        assert_eq!(parse_css_color("#f00"), Some(Color::from_rgb(255, 0, 0)));
        assert_eq!(parse_css_color("red"), Some(Color::from_rgb(255, 0, 0)));
        assert_eq!(
            parse_css_color("rgb(255, 128, 0)"),
            Some(Color::from_rgb(255, 128, 0))
        );
    }

    #[test]
    fn test_parse_declarations() {
        let decls = parse_declarations("fill: red; stroke: blue; stroke-width: 2px");
        assert_eq!(decls.get("fill"), Some(&"red".to_string()));
        assert_eq!(decls.get("stroke"), Some(&"blue".to_string()));
        assert_eq!(decls.get("stroke-width"), Some(&"2px".to_string()));
    }

    #[test]
    fn test_parse_stylesheet() {
        let css = r#"
            rect { fill: red; }
            .highlight { stroke: yellow; }
            #main { opacity: 0.5; }
        "#;

        let stylesheet = Stylesheet::parse(css);
        assert_eq!(stylesheet.rules.len(), 3);
    }

    #[test]
    fn test_selector_parse() {
        assert!(matches!(
            CssSelector::parse("rect"),
            CssSelector::Element(_)
        ));
        assert!(matches!(
            CssSelector::parse(".class"),
            CssSelector::Class(_)
        ));
        assert!(matches!(CssSelector::parse("#id"), CssSelector::Id(_)));
    }

    #[test]
    fn test_selector_specificity() {
        let elem = CssSelector::parse("rect");
        let class = CssSelector::parse(".highlight");
        let id = CssSelector::parse("#main");

        assert_eq!(elem.specificity(), (0, 0, 1));
        assert_eq!(class.specificity(), (0, 1, 0));
        assert_eq!(id.specificity(), (1, 0, 0));
    }

    #[test]
    fn test_descendant_selector() {
        let sel = CssSelector::parse("g rect");
        assert!(matches!(sel, CssSelector::Descendant(_, _)));
    }

    #[test]
    fn test_hsl_color() {
        let color = parse_css_color("hsl(0, 100%, 50%)").unwrap();
        // Should be red
        assert_eq!(color.red(), 255);
        assert!(color.green() < 10);
        assert!(color.blue() < 10);
    }
}
