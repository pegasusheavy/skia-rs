//! Blend modes for compositing.

/// Porter-Duff and other blend modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum BlendMode {
    // Porter-Duff modes
    /// Clear destination.
    Clear = 0,
    /// Source only.
    Src,
    /// Destination only.
    Dst,
    /// Source over destination (default).
    #[default]
    SrcOver,
    /// Destination over source.
    DstOver,
    /// Source where destination exists.
    SrcIn,
    /// Destination where source exists.
    DstIn,
    /// Source where destination is empty.
    SrcOut,
    /// Destination where source is empty.
    DstOut,
    /// Source on top of destination.
    SrcATop,
    /// Destination on top of source.
    DstATop,
    /// XOR of source and destination.
    Xor,
    /// Sum of source and destination.
    Plus,
    /// Product of source and destination.
    Modulate,

    // Separable blend modes
    /// Screen blend.
    Screen,
    /// Overlay blend.
    Overlay,
    /// Darken (minimum).
    Darken,
    /// Lighten (maximum).
    Lighten,
    /// Color dodge.
    ColorDodge,
    /// Color burn.
    ColorBurn,
    /// Hard light.
    HardLight,
    /// Soft light.
    SoftLight,
    /// Difference.
    Difference,
    /// Exclusion.
    Exclusion,
    /// Multiply.
    Multiply,

    // Non-separable blend modes
    /// Hue blend.
    Hue,
    /// Saturation blend.
    Saturation,
    /// Color blend.
    Color,
    /// Luminosity blend.
    Luminosity,
}

impl BlendMode {
    /// Get the name of the blend mode.
    pub const fn name(&self) -> &'static str {
        match self {
            BlendMode::Clear => "Clear",
            BlendMode::Src => "Src",
            BlendMode::Dst => "Dst",
            BlendMode::SrcOver => "SrcOver",
            BlendMode::DstOver => "DstOver",
            BlendMode::SrcIn => "SrcIn",
            BlendMode::DstIn => "DstIn",
            BlendMode::SrcOut => "SrcOut",
            BlendMode::DstOut => "DstOut",
            BlendMode::SrcATop => "SrcATop",
            BlendMode::DstATop => "DstATop",
            BlendMode::Xor => "Xor",
            BlendMode::Plus => "Plus",
            BlendMode::Modulate => "Modulate",
            BlendMode::Screen => "Screen",
            BlendMode::Overlay => "Overlay",
            BlendMode::Darken => "Darken",
            BlendMode::Lighten => "Lighten",
            BlendMode::ColorDodge => "ColorDodge",
            BlendMode::ColorBurn => "ColorBurn",
            BlendMode::HardLight => "HardLight",
            BlendMode::SoftLight => "SoftLight",
            BlendMode::Difference => "Difference",
            BlendMode::Exclusion => "Exclusion",
            BlendMode::Multiply => "Multiply",
            BlendMode::Hue => "Hue",
            BlendMode::Saturation => "Saturation",
            BlendMode::Color => "Color",
            BlendMode::Luminosity => "Luminosity",
        }
    }

    /// Create a blend mode from a u8 value.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(BlendMode::Clear),
            1 => Some(BlendMode::Src),
            2 => Some(BlendMode::Dst),
            3 => Some(BlendMode::SrcOver),
            4 => Some(BlendMode::DstOver),
            5 => Some(BlendMode::SrcIn),
            6 => Some(BlendMode::DstIn),
            7 => Some(BlendMode::SrcOut),
            8 => Some(BlendMode::DstOut),
            9 => Some(BlendMode::SrcATop),
            10 => Some(BlendMode::DstATop),
            11 => Some(BlendMode::Xor),
            12 => Some(BlendMode::Plus),
            13 => Some(BlendMode::Modulate),
            14 => Some(BlendMode::Screen),
            15 => Some(BlendMode::Overlay),
            16 => Some(BlendMode::Darken),
            17 => Some(BlendMode::Lighten),
            18 => Some(BlendMode::ColorDodge),
            19 => Some(BlendMode::ColorBurn),
            20 => Some(BlendMode::HardLight),
            21 => Some(BlendMode::SoftLight),
            22 => Some(BlendMode::Difference),
            23 => Some(BlendMode::Exclusion),
            24 => Some(BlendMode::Multiply),
            25 => Some(BlendMode::Hue),
            26 => Some(BlendMode::Saturation),
            27 => Some(BlendMode::Color),
            28 => Some(BlendMode::Luminosity),
            _ => None,
        }
    }

    /// Check if this is a Porter-Duff mode.
    #[inline]
    pub fn is_porter_duff(&self) -> bool {
        (*self as u8) <= (BlendMode::Modulate as u8)
    }

    /// Check if this is a separable blend mode.
    #[inline]
    pub fn is_separable(&self) -> bool {
        let v = *self as u8;
        v >= (BlendMode::Screen as u8) && v <= (BlendMode::Multiply as u8)
    }

    /// Check if this is a non-separable blend mode.
    #[inline]
    pub fn is_non_separable(&self) -> bool {
        let v = *self as u8;
        v >= (BlendMode::Hue as u8) && v <= (BlendMode::Luminosity as u8)
    }
}
