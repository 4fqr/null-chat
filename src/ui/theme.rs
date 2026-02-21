use iced::{
    widget::{button, container, text_input},
    Background, Border, Color, Theme, Vector,
};

pub const BACKGROUND: Color      = Color { r: 0.000, g: 0.000, b: 0.000, a: 1.0 };
pub const SURFACE_0: Color       = Color { r: 0.039, g: 0.039, b: 0.039, a: 1.0 };
pub const SURFACE_1: Color       = Color { r: 0.063, g: 0.063, b: 0.063, a: 1.0 };
pub const SURFACE_2: Color       = Color { r: 0.094, g: 0.094, b: 0.094, a: 1.0 };
pub const BORDER: Color          = Color { r: 0.150, g: 0.150, b: 0.150, a: 1.0 };
pub const BORDER_BRIGHT: Color   = Color { r: 0.260, g: 0.260, b: 0.260, a: 1.0 };
pub const TEXT_PRIMARY: Color    = Color { r: 0.800, g: 0.800, b: 0.800, a: 1.0 };
pub const TEXT_DIM: Color        = Color { r: 0.360, g: 0.360, b: 0.360, a: 1.0 };
pub const TEXT_BRIGHT: Color     = Color { r: 0.970, g: 0.970, b: 0.970, a: 1.0 };
pub const STATUS_SECURE: Color   = Color { r: 0.000, g: 0.760, b: 0.000, a: 1.0 };
pub const STATUS_PENDING: Color  = Color { r: 0.920, g: 0.560, b: 0.000, a: 1.0 };
pub const STATUS_INSECURE: Color = Color { r: 0.820, g: 0.000, b: 0.000, a: 1.0 };
pub const ACCENT: Color          = Color { r: 0.000, g: 0.720, b: 0.000, a: 1.0 };

fn no_border() -> Border {
    Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() }
}

fn line(color: Color) -> Border {
    Border { color, width: 1.0, radius: 0.0.into() }
}

pub struct BgBlack;
impl container::StyleSheet for BgBlack {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(BACKGROUND)),
            border: no_border(),
            text_color: Some(TEXT_PRIMARY),
            ..Default::default()
        }
    }
}

pub struct StatusBarStyle;
impl container::StyleSheet for StatusBarStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(SURFACE_0)),
            border: line(BORDER),
            text_color: Some(TEXT_DIM),
            ..Default::default()
        }
    }
}

pub struct PanelLeft;
impl container::StyleSheet for PanelLeft {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(SURFACE_0)),
            border: line(BORDER),
            text_color: Some(TEXT_PRIMARY),
            ..Default::default()
        }
    }
}

pub struct PanelMid;
impl container::StyleSheet for PanelMid {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(SURFACE_0)),
            border: line(BORDER),
            text_color: Some(TEXT_PRIMARY),
            ..Default::default()
        }
    }
}

pub struct PanelRight;
impl container::StyleSheet for PanelRight {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(BACKGROUND)),
            border: no_border(),
            text_color: Some(TEXT_PRIMARY),
            ..Default::default()
        }
    }
}

pub struct MessageHeaderStyle;
impl container::StyleSheet for MessageHeaderStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(SURFACE_0)),
            border: line(BORDER),
            text_color: Some(TEXT_BRIGHT),
            ..Default::default()
        }
    }
}

pub struct ComposeBarStyle;
impl container::StyleSheet for ComposeBarStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(SURFACE_0)),
            border: line(BORDER),
            text_color: Some(TEXT_PRIMARY),
            ..Default::default()
        }
    }
}

pub struct UnlockCardStyle;
impl container::StyleSheet for UnlockCardStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(SURFACE_0)),
            border: line(BORDER_BRIGHT),
            text_color: Some(TEXT_PRIMARY),
            ..Default::default()
        }
    }
}

pub struct HighlightedRow;
impl container::StyleSheet for HighlightedRow {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(SURFACE_2)),
            border: Border { color: ACCENT, width: 1.0, radius: 0.0.into() },
            text_color: Some(TEXT_BRIGHT),
            ..Default::default()
        }
    }
}

pub struct FlatButton;
impl button::StyleSheet for FlatButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            border: no_border(),
            text_color: TEXT_PRIMARY,
            shadow_offset: Vector::new(0.0, 0.0),
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(SURFACE_1)),
            border: no_border(),
            text_color: TEXT_BRIGHT,
            shadow_offset: Vector::new(0.0, 0.0),
            ..Default::default()
        }
    }
}

pub struct ActiveFlatButton;
impl button::StyleSheet for ActiveFlatButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(SURFACE_2)),
            border: Border { color: ACCENT, width: 1.0, radius: 0.0.into() },
            text_color: TEXT_BRIGHT,
            shadow_offset: Vector::new(0.0, 0.0),
            ..Default::default()
        }
    }
    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }
    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }
}

pub struct SendButtonStyle;
impl button::StyleSheet for SendButtonStyle {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(ACCENT)),
            border: no_border(),
            text_color: Color::BLACK,
            shadow_offset: Vector::new(0.0, 0.0),
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(STATUS_SECURE)),
            border: no_border(),
            text_color: Color::BLACK,
            shadow_offset: Vector::new(0.0, 0.0),
            ..Default::default()
        }
    }
}

pub struct AccentButton;
impl button::StyleSheet for AccentButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(ACCENT)),
            border: no_border(),
            text_color: Color::BLACK,
            shadow_offset: Vector::new(0.0, 0.0),
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(STATUS_SECURE)),
            border: no_border(),
            text_color: Color::BLACK,
            shadow_offset: Vector::new(0.0, 0.0),
            ..Default::default()
        }
    }
}

pub struct DarkInputStyle;
impl text_input::StyleSheet for DarkInputStyle {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(SURFACE_1),
            border: line(BORDER),
            icon_color: TEXT_DIM,
        }
    }
    fn focused(&self, _: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(SURFACE_1),
            border: line(BORDER_BRIGHT),
            icon_color: TEXT_PRIMARY,
        }
    }
    fn hovered(&self, style: &Self::Style) -> text_input::Appearance {
        self.focused(style)
    }
    fn placeholder_color(&self, _: &Self::Style) -> Color {
        TEXT_DIM
    }
    fn value_color(&self, _: &Self::Style) -> Color {
        TEXT_BRIGHT
    }
    fn selection_color(&self, _: &Self::Style) -> Color {
        SURFACE_2
    }
    fn disabled_color(&self, _: &Self::Style) -> Color {
        TEXT_DIM
    }
    fn disabled(&self, _: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(SURFACE_0),
            border: line(BORDER),
            icon_color: TEXT_DIM,
        }
    }
}
