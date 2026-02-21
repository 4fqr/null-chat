/// Discord-like color palette and iced StyleSheet implementations.
use iced::{
    widget::{button, container, text_input},
    Background, Border, Color, Shadow, Theme, Vector,
};

// ─── Discord Color Palette ────────────────────────────────────────────────────

pub const BG_DARKEST: Color = Color { r: 0.125, g: 0.133, b: 0.145, a: 1.0 };
pub const BG_DARK: Color    = Color { r: 0.184, g: 0.192, b: 0.212, a: 1.0 };
pub const BG_MAIN: Color    = Color { r: 0.212, g: 0.224, b: 0.247, a: 1.0 };
pub const BG_INPUT: Color   = Color { r: 0.251, g: 0.267, b: 0.294, a: 1.0 };
pub const BG_HOVER: Color   = Color { r: 0.196, g: 0.208, b: 0.231, a: 1.0 };
pub const BG_SELECTED: Color= Color { r: 0.235, g: 0.247, b: 0.271, a: 1.0 };
pub const DIVIDER: Color    = Color { r: 0.310, g: 0.329, b: 0.361, a: 1.0 };

pub const TEXT_NORMAL: Color = Color { r: 0.863, g: 0.867, b: 0.871, a: 1.0 };
pub const TEXT_MUTED: Color  = Color { r: 0.557, g: 0.573, b: 0.592, a: 1.0 };
pub const TEXT_WHITE: Color  = Color { r: 1.0,   g: 1.0,   b: 1.0,   a: 1.0 };
pub const TEXT_MEDIUM: Color = Color { r: 0.725, g: 0.733, b: 0.745, a: 1.0 };

pub const BLURPLE: Color      = Color { r: 0.345, g: 0.396, b: 0.949, a: 1.0 };
pub const BLURPLE_HOVER: Color= Color { r: 0.278, g: 0.322, b: 0.769, a: 1.0 };
pub const GREEN: Color        = Color { r: 0.231, g: 0.647, b: 0.365, a: 1.0 };
pub const RED: Color          = Color { r: 0.929, g: 0.259, b: 0.271, a: 1.0 };
pub const YELLOW: Color       = Color { r: 0.980, g: 0.659, b: 0.102, a: 1.0 };

// Legacy aliases
pub const BACKGROUND: Color    = BG_DARKEST;
pub const TEXT_PRIMARY: Color  = TEXT_NORMAL;
pub const TEXT_DIM: Color      = TEXT_MUTED;
pub const TEXT_BRIGHT: Color   = TEXT_WHITE;
pub const STATUS_SECURE: Color = GREEN;
pub const STATUS_PENDING: Color= YELLOW;
pub const STATUS_INSECURE: Color= RED;
pub const ACCENT: Color        = BLURPLE;

// ─── Border helpers ───────────────────────────────────────────────────────────

pub fn no_border() -> Border {
    Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() }
}

fn rounded(r: f32) -> Border {
    Border { color: Color::TRANSPARENT, width: 0.0, radius: r.into() }
}

fn colored_border(c: Color, w: f32, r: f32) -> Border {
    Border { color: c, width: w, radius: r.into() }
}

// ─── Container Styles ─────────────────────────────────────────────────────────

macro_rules! bg_container {
    ($name:ident, $bg:expr, $text:expr) => {
        pub struct $name;
        impl container::StyleSheet for $name {
            type Style = Theme;
            fn appearance(&self, _: &Self::Style) -> container::Appearance {
                container::Appearance {
                    background: Some(Background::Color($bg)),
                    border: no_border(),
                    text_color: Some($text),
                    ..Default::default()
                }
            }
        }
    };
}

bg_container!(BgDarkest, BG_DARKEST, TEXT_NORMAL);
bg_container!(BgDark,    BG_DARK,    TEXT_NORMAL);
bg_container!(BgMain,    BG_MAIN,    TEXT_NORMAL);
bg_container!(BgBlack,   BG_DARKEST, TEXT_NORMAL);
bg_container!(PanelLeft, BG_DARK,    TEXT_NORMAL);
bg_container!(PanelMid,  BG_DARK,    TEXT_NORMAL);
bg_container!(PanelRight,BG_MAIN,    TEXT_NORMAL);
bg_container!(HoverRow,  BG_HOVER,   TEXT_NORMAL);

pub struct StatusBarStyle;
impl container::StyleSheet for StatusBarStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(BG_DARKEST)),
            border: colored_border(DIVIDER, 1.0, 0.0),
            text_color: Some(TEXT_MUTED),
            ..Default::default()
        }
    }
}

pub struct CardStyle;
impl container::StyleSheet for CardStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(BG_DARK)),
            border: colored_border(DIVIDER, 1.0, 8.0),
            text_color: Some(TEXT_NORMAL),
            shadow: Shadow {
                color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.4 },
                offset: Vector::new(0.0, 4.0),
                blur_radius: 16.0,
            },
        }
    }
}

pub struct UnlockCardStyle;
impl container::StyleSheet for UnlockCardStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(BG_DARK)),
            border: colored_border(DIVIDER, 1.0, 8.0),
            text_color: Some(TEXT_NORMAL),
            shadow: Shadow {
                color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.5 },
                offset: Vector::new(0.0, 8.0),
                blur_radius: 24.0,
            },
        }
    }
}

pub struct MessageHeaderStyle;
impl container::StyleSheet for MessageHeaderStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(BG_MAIN)),
            border: Border { color: DIVIDER, width: 1.0, radius: 0.0.into() },
            text_color: Some(TEXT_WHITE),
            shadow: Shadow {
                color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.2 },
                offset: Vector::new(0.0, 2.0),
                blur_radius: 4.0,
            },
        }
    }
}

pub struct ComposeBarStyle;
impl container::StyleSheet for ComposeBarStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(BG_INPUT)),
            border: rounded(8.0),
            ..Default::default()
        }
    }
}

pub struct AvatarContainer {
    pub color: Color,
    pub radius: f32,
}
impl container::StyleSheet for AvatarContainer {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.color)),
            border: rounded(self.radius),
            text_color: Some(TEXT_WHITE),
            ..Default::default()
        }
    }
}

pub struct TagBlurple;
impl container::StyleSheet for TagBlurple {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color { r: 0.345, g: 0.396, b: 0.949, a: 0.15 })),
            border: colored_border(Color { r: 0.345, g: 0.396, b: 0.949, a: 0.4 }, 1.0, 4.0),
            text_color: Some(BLURPLE),
            ..Default::default()
        }
    }
}

pub struct TagGreen;
impl container::StyleSheet for TagGreen {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color { r: 0.231, g: 0.647, b: 0.365, a: 0.15 })),
            border: colored_border(Color { r: 0.231, g: 0.647, b: 0.365, a: 0.4 }, 1.0, 4.0),
            text_color: Some(GREEN),
            ..Default::default()
        }
    }
}

// ─── Button Styles ────────────────────────────────────────────────────────────

pub struct FlatButton;
impl button::StyleSheet for FlatButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border: rounded(4.0),
            text_color: TEXT_MUTED,
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BG_HOVER)),
            border: rounded(4.0),
            text_color: TEXT_NORMAL,
            ..Default::default()
        }
    }
}

pub struct ActiveFlatButton;
impl button::StyleSheet for ActiveFlatButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BG_SELECTED)),
            border: rounded(4.0),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BG_SELECTED)),
            border: rounded(4.0),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
}

pub struct BlurpleButton;
impl button::StyleSheet for BlurpleButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BLURPLE)),
            border: rounded(4.0),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BLURPLE_HOVER)),
            border: rounded(4.0),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
}

pub use BlurpleButton as AccentButton;
pub use BlurpleButton as SendButtonStyle;

pub struct DangerButton;
impl button::StyleSheet for DangerButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(RED)),
            border: rounded(4.0),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color { r: 0.8, g: 0.2, b: 0.22, a: 1.0 })),
            border: rounded(4.0),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
}

pub struct GhostButton;
impl button::StyleSheet for GhostButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border: colored_border(DIVIDER, 1.0, 4.0),
            text_color: TEXT_NORMAL,
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BG_HOVER)),
            border: colored_border(TEXT_MUTED, 1.0, 4.0),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
}

pub struct ServerIconButton;
impl button::StyleSheet for ServerIconButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BG_DARK)),
            border: rounded(16.0),
            text_color: TEXT_NORMAL,
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BLURPLE)),
            border: rounded(8.0),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
}

pub struct ActiveServerIconButton;
impl button::StyleSheet for ActiveServerIconButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BLURPLE)),
            border: rounded(8.0),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BLURPLE_HOVER)),
            border: rounded(8.0),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
}

// ─── Text Input ───────────────────────────────────────────────────────────────

pub struct DiscordInput;
impl text_input::StyleSheet for DiscordInput {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(BG_INPUT),
            border: colored_border(Color::TRANSPARENT, 0.0, 4.0),
            icon_color: TEXT_MUTED,
        }
    }
    fn focused(&self, _: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(BG_INPUT),
            border: colored_border(BLURPLE, 2.0, 4.0),
            icon_color: TEXT_MUTED,
        }
    }
    fn placeholder_color(&self, _: &Self::Style) -> Color { TEXT_MUTED }
    fn value_color(&self, _: &Self::Style) -> Color { TEXT_WHITE }
    fn selection_color(&self, _: &Self::Style) -> Color {
        Color { r: 0.345, g: 0.396, b: 0.949, a: 0.4 }
    }
    fn disabled_color(&self, _: &Self::Style) -> Color { TEXT_MUTED }
    fn disabled(&self, _: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(BG_DARK),
            border: colored_border(DIVIDER, 1.0, 4.0),
            icon_color: TEXT_MUTED,
        }
    }
}

pub use DiscordInput as DarkInputStyle;
