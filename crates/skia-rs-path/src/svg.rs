//! SVG path data parsing.
//!
//! Parses SVG path `d` attribute strings into a `Path`.

use crate::{Path, PathBuilder};
use skia_rs_core::Scalar;

/// Parse an SVG path data string.
///
/// # Example
/// ```
/// use skia_rs_path::parse_svg_path;
///
/// let path = parse_svg_path("M 10 10 L 100 100 Z").unwrap();
/// assert!(!path.is_empty());
/// ```
pub fn parse_svg_path(d: &str) -> Result<Path, SvgPathError> {
    let parser = SvgPathParser::new(d);
    parser.parse()
}

/// Error type for SVG path parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum SvgPathError {
    /// Unexpected end of input.
    UnexpectedEnd,
    /// Invalid number format.
    InvalidNumber(String),
    /// Unknown command.
    UnknownCommand(char),
    /// Expected a number.
    ExpectedNumber,
    /// Missing move command at start.
    MissingMoveTo,
}

impl std::fmt::Display for SvgPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SvgPathError::UnexpectedEnd => write!(f, "unexpected end of path data"),
            SvgPathError::InvalidNumber(s) => write!(f, "invalid number: {}", s),
            SvgPathError::UnknownCommand(c) => write!(f, "unknown command: {}", c),
            SvgPathError::ExpectedNumber => write!(f, "expected a number"),
            SvgPathError::MissingMoveTo => write!(f, "path must start with moveto"),
        }
    }
}

impl std::error::Error for SvgPathError {}

struct SvgPathParser<'a> {
    input: &'a str,
    pos: usize,
    builder: PathBuilder,
    last_control: Option<(Scalar, Scalar)>,
    has_move: bool,
}

impl<'a> SvgPathParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            builder: PathBuilder::new(),
            last_control: None,
            has_move: false,
        }
    }

    fn parse(mut self) -> Result<Path, SvgPathError> {
        self.skip_whitespace();

        while !self.is_end() {
            let cmd = self.parse_command()?;
            self.execute_command(cmd)?;
            self.skip_whitespace();
        }

        Ok(self.builder.build())
    }

    fn is_end(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += c.len_utf8();
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() || c == ',' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn parse_command(&mut self) -> Result<char, SvgPathError> {
        self.skip_whitespace();
        let cmd = self.peek().ok_or(SvgPathError::UnexpectedEnd)?;

        if cmd.is_ascii_alphabetic() {
            self.advance();
            Ok(cmd)
        } else if !self.has_move {
            Err(SvgPathError::MissingMoveTo)
        } else {
            // Implicit lineto
            Ok('L')
        }
    }

    fn parse_number(&mut self) -> Result<Scalar, SvgPathError> {
        self.skip_whitespace();

        let start = self.pos;
        let mut has_dot = false;
        let mut has_exp = false;

        // Handle sign
        if let Some(c) = self.peek() {
            if c == '+' || c == '-' {
                self.advance();
            }
        }

        // Parse digits and decimal point
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else if c == '.' && !has_dot && !has_exp {
                has_dot = true;
                self.advance();
            } else if (c == 'e' || c == 'E') && !has_exp {
                has_exp = true;
                self.advance();
                // Handle exponent sign
                if let Some(next) = self.peek() {
                    if next == '+' || next == '-' {
                        self.advance();
                    }
                }
            } else {
                break;
            }
        }

        if start == self.pos {
            return Err(SvgPathError::ExpectedNumber);
        }

        let num_str = &self.input[start..self.pos];
        num_str
            .parse()
            .map_err(|_| SvgPathError::InvalidNumber(num_str.to_string()))
    }

    fn parse_flag(&mut self) -> Result<bool, SvgPathError> {
        self.skip_whitespace();
        match self.peek() {
            Some('0') => {
                self.advance();
                Ok(false)
            }
            Some('1') => {
                self.advance();
                Ok(true)
            }
            _ => Err(SvgPathError::ExpectedNumber),
        }
    }

    fn current_point(&self) -> (Scalar, Scalar) {
        let p = self.builder.current_point();
        (p.x, p.y)
    }

    fn execute_command(&mut self, cmd: char) -> Result<(), SvgPathError> {
        let is_relative = cmd.is_ascii_lowercase();
        let cmd_upper = cmd.to_ascii_uppercase();

        match cmd_upper {
            'M' => self.parse_moveto(is_relative),
            'L' => self.parse_lineto(is_relative),
            'H' => self.parse_horizontal_lineto(is_relative),
            'V' => self.parse_vertical_lineto(is_relative),
            'C' => self.parse_curveto(is_relative),
            'S' => self.parse_smooth_curveto(is_relative),
            'Q' => self.parse_quadto(is_relative),
            'T' => self.parse_smooth_quadto(is_relative),
            'A' => self.parse_arcto(is_relative),
            'Z' => {
                self.builder.close();
                self.last_control = None;
                Ok(())
            }
            _ => Err(SvgPathError::UnknownCommand(cmd)),
        }
    }

    fn parse_moveto(&mut self, is_relative: bool) -> Result<(), SvgPathError> {
        let mut first = true;
        loop {
            self.skip_whitespace();
            if self.is_end() || self.peek().map_or(false, |c| c.is_ascii_alphabetic()) {
                break;
            }

            let x = self.parse_number()?;
            let y = self.parse_number()?;

            let (x, y) = if is_relative && self.has_move {
                let (cx, cy) = self.current_point();
                (cx + x, cy + y)
            } else {
                (x, y)
            };

            if first {
                self.builder.move_to(x, y);
                self.has_move = true;
                first = false;
            } else {
                self.builder.line_to(x, y);
            }
        }
        self.last_control = None;
        Ok(())
    }

    fn parse_lineto(&mut self, is_relative: bool) -> Result<(), SvgPathError> {
        loop {
            self.skip_whitespace();
            if self.is_end() || self.peek().map_or(false, |c| c.is_ascii_alphabetic()) {
                break;
            }

            let x = self.parse_number()?;
            let y = self.parse_number()?;

            let (x, y) = if is_relative {
                let (cx, cy) = self.current_point();
                (cx + x, cy + y)
            } else {
                (x, y)
            };

            self.builder.line_to(x, y);
        }
        self.last_control = None;
        Ok(())
    }

    fn parse_horizontal_lineto(&mut self, is_relative: bool) -> Result<(), SvgPathError> {
        loop {
            self.skip_whitespace();
            if self.is_end() || self.peek().map_or(false, |c| c.is_ascii_alphabetic()) {
                break;
            }

            let x = self.parse_number()?;
            let (cx, cy) = self.current_point();

            let x = if is_relative { cx + x } else { x };
            self.builder.line_to(x, cy);
        }
        self.last_control = None;
        Ok(())
    }

    fn parse_vertical_lineto(&mut self, is_relative: bool) -> Result<(), SvgPathError> {
        loop {
            self.skip_whitespace();
            if self.is_end() || self.peek().map_or(false, |c| c.is_ascii_alphabetic()) {
                break;
            }

            let y = self.parse_number()?;
            let (cx, cy) = self.current_point();

            let y = if is_relative { cy + y } else { y };
            self.builder.line_to(cx, y);
        }
        self.last_control = None;
        Ok(())
    }

    fn parse_curveto(&mut self, is_relative: bool) -> Result<(), SvgPathError> {
        loop {
            self.skip_whitespace();
            if self.is_end() || self.peek().map_or(false, |c| c.is_ascii_alphabetic()) {
                break;
            }

            let x1 = self.parse_number()?;
            let y1 = self.parse_number()?;
            let x2 = self.parse_number()?;
            let y2 = self.parse_number()?;
            let x = self.parse_number()?;
            let y = self.parse_number()?;

            let (x1, y1, x2, y2, x, y) = if is_relative {
                let (cx, cy) = self.current_point();
                (cx + x1, cy + y1, cx + x2, cy + y2, cx + x, cy + y)
            } else {
                (x1, y1, x2, y2, x, y)
            };

            self.builder.cubic_to(x1, y1, x2, y2, x, y);
            self.last_control = Some((x2, y2));
        }
        Ok(())
    }

    fn parse_smooth_curveto(&mut self, is_relative: bool) -> Result<(), SvgPathError> {
        loop {
            self.skip_whitespace();
            if self.is_end() || self.peek().map_or(false, |c| c.is_ascii_alphabetic()) {
                break;
            }

            let x2 = self.parse_number()?;
            let y2 = self.parse_number()?;
            let x = self.parse_number()?;
            let y = self.parse_number()?;

            let (cx, cy) = self.current_point();
            let (x1, y1) = if let Some((lx, ly)) = self.last_control {
                (2.0 * cx - lx, 2.0 * cy - ly)
            } else {
                (cx, cy)
            };

            let (x2, y2, x, y) = if is_relative {
                (cx + x2, cy + y2, cx + x, cy + y)
            } else {
                (x2, y2, x, y)
            };

            self.builder.cubic_to(x1, y1, x2, y2, x, y);
            self.last_control = Some((x2, y2));
        }
        Ok(())
    }

    fn parse_quadto(&mut self, is_relative: bool) -> Result<(), SvgPathError> {
        loop {
            self.skip_whitespace();
            if self.is_end() || self.peek().map_or(false, |c| c.is_ascii_alphabetic()) {
                break;
            }

            let x1 = self.parse_number()?;
            let y1 = self.parse_number()?;
            let x = self.parse_number()?;
            let y = self.parse_number()?;

            let (x1, y1, x, y) = if is_relative {
                let (cx, cy) = self.current_point();
                (cx + x1, cy + y1, cx + x, cy + y)
            } else {
                (x1, y1, x, y)
            };

            self.builder.quad_to(x1, y1, x, y);
            self.last_control = Some((x1, y1));
        }
        Ok(())
    }

    fn parse_smooth_quadto(&mut self, is_relative: bool) -> Result<(), SvgPathError> {
        loop {
            self.skip_whitespace();
            if self.is_end() || self.peek().map_or(false, |c| c.is_ascii_alphabetic()) {
                break;
            }

            let x = self.parse_number()?;
            let y = self.parse_number()?;

            let (cx, cy) = self.current_point();
            let (x1, y1) = if let Some((lx, ly)) = self.last_control {
                (2.0 * cx - lx, 2.0 * cy - ly)
            } else {
                (cx, cy)
            };

            let (x, y) = if is_relative { (cx + x, cy + y) } else { (x, y) };

            self.builder.quad_to(x1, y1, x, y);
            self.last_control = Some((x1, y1));
        }
        Ok(())
    }

    fn parse_arcto(&mut self, is_relative: bool) -> Result<(), SvgPathError> {
        loop {
            self.skip_whitespace();
            if self.is_end() || self.peek().map_or(false, |c| c.is_ascii_alphabetic()) {
                break;
            }

            let rx = self.parse_number()?;
            let ry = self.parse_number()?;
            let x_rotation = self.parse_number()?;
            let large_arc = self.parse_flag()?;
            let sweep = self.parse_flag()?;
            let x = self.parse_number()?;
            let y = self.parse_number()?;

            let (x, y) = if is_relative {
                let (cx, cy) = self.current_point();
                (cx + x, cy + y)
            } else {
                (x, y)
            };

            self.builder.arc_to(rx, ry, x_rotation, large_arc, sweep, x, y);
        }
        self.last_control = None;
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_path() {
        let path = parse_svg_path("M 10 20 L 30 40 Z").unwrap();
        assert_eq!(path.verb_count(), 3); // Move, Line, Close
    }

    #[test]
    fn test_parse_relative_commands() {
        let path = parse_svg_path("M 10 20 l 20 20 z").unwrap();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_parse_curves() {
        let path = parse_svg_path("M 0 0 C 10 20 30 40 50 60").unwrap();
        assert_eq!(path.verb_count(), 2); // Move, Cubic
    }

    #[test]
    fn test_parse_arc() {
        let path = parse_svg_path("M 0 0 A 50 50 0 0 1 100 0").unwrap();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_parse_horizontal_vertical() {
        let path = parse_svg_path("M 0 0 H 100 V 100 H 0 Z").unwrap();
        assert_eq!(path.verb_count(), 5);
    }
}
