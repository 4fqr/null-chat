/// Premium Discord-style palette and iced StyleSheet implementations.
/// Designed to feel smooth, modern, and information-dense.
use iced::{
    widget::{button, container, text_input, scrollable},
    Background, Border, Color, Shadow, Theme, Vector,
};

// ─── Discord Color Palette ────────────────────────────────────────────────────

pub const BG_DARKEST: Color  = Color { r: 0.125, g: 0.133, b: 0.145, a: 1.0 }; // #202225
pub const BG_DARK: Color     = Color { r: 0.184, g: 0.192, b: 0.212, a: 1.0 }; // #2f3136
pub const BG_MAIN: Color     = Color { r: 0.212, g: 0.224, b: 0.247, a: 1.0 }; // #36393f
pub const BG_INPUT: Color    = Color { r: 0.251, g: 0.267, b: 0.294, a: 1.0 }; // #40444b
pub const BG_HOVER: Color    = Color { r: 0.220, g: 0.231, b: 0.255, a: 1.0 };
pub const BG_SELECTED: Color = Color { r: 0.251, g: 0.263, b: 0.290, a: 1.0 };
pub const BG_MODAL: Color    = Color { r: 0.141, g: 0.149, b: 0.165, a: 1.0 }; // #242529
pub const BG_POPUP: Color    = Color { r: 0.173, g: 0.180, b: 0.200, a: 1.0 };
pub const DIVIDER: Color     = Color { r: 0.165, g: 0.173, b: 0.192, a: 1.0 };

pub const TEXT_NORMAL: Color = Color { r: 0.863, g: 0.867, b: 0.871, a: 1.0 }; // #dcddde
pub const TEXT_MUTED: Color  = Color { r: 0.557, g: 0.573, b: 0.592, a: 1.0 }; // #8e9297
pub const TEXT_WHITE: Color  = Color { r: 1.0,   g: 1.0,   b: 1.0,   a: 1.0 };
pub const TEXT_MEDIUM: Color = Color { r: 0.725, g: 0.733, b: 0.745, a: 1.0 };
pub const TEXT_LINK: Color   = Color { r: 0.400, g: 0.537, b: 0.965, a: 1.0 };

pub const BLURPLE: Color       = Color { r: 0.345, g: 0.396, b: 0.949, a: 1.0 }; // #5865F2
pub const BLURPLE_DARK: Color  = Color { r: 0.278, g: 0.322, b: 0.769, a: 1.0 }; // #4752c4
pub const BLURPLE_LIGHT: Color = Color { r: 0.435, g: 0.482, b: 0.965, a: 1.0 };
pub const GREEN: Color         = Color { r: 0.231, g: 0.647, b: 0.365, a: 1.0 }; // #3ba55d
pub const GREEN_DARK: Color    = Color { r: 0.180, g: 0.510, b: 0.286, a: 1.0 };
pub const RED: Color           = Color { r: 0.929, g: 0.259, b: 0.271, a: 1.0 }; // #ed4245
pub const RED_DARK: Color      = Color { r: 0.718, g: 0.200, b: 0.208, a: 1.0 };
pub const YELLOW: Color        = Color { r: 0.980, g: 0.659, b: 0.102, a: 1.0 }; // #faa81a
pub const YELLOW_DARK: Color   = Color { r: 0.780, g: 0.520, b: 0.075, a: 1.0 };
pub const ORANGE: Color        = Color { r: 0.988, g: 0.447, b: 0.243, a: 1.0 };

// Legacy compat
pub const BACKGROUND: Color     = BG_DARKEST;
pub const TEXT_PRIMARY: Color   = TEXT_NORMAL;
pub const TEXT_DIM: Color       = TEXT_MUTED;
pub const TEXT_BRIGHT: Color    = TEXT_WHITE;
pub const STATUS_SECURE: Color  = GREEN;
pub const STATUS_PENDING: Color = YELLOW;
pub const STATUS_INSECURE: Color= RED;
pub const ACCENT: Color         = BLURPLE;
pub const BG_BLACK: Color       = BG_DARKEST;

// ─── Border helpers ───────────────────────────────────────────────────────────

pub fn no_border() -> Border {
    Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() }
}

pub fn rounded(r: f32) -> Border {
    Border { color: Color::TRANSPARENT, width: 0.0, radius: r.into() }
}

pub fn colored_border(c: Color, w: f32, r: f32) -> Border {
    Border { color: c, width: w, radius: r.into() }
}

pub fn shadow(size: f32, offset_y: f32, alpha: f32) -> Shadow {
    Shadow {
        color: Color { r: 0.0, g: 0.0, b: 0.0, a: alpha },
        offset: Vector::new(0.0, offset_y),
        blur_radius: size,
    }
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
                    shadow: Shadow::default(),
                }
            }
        }
    };
}

bg_container!(BgDarkest,  BG_DARKEST, TEXT_NORMAL);
bg_container!(BgDark,     BG_DARK,    TEXT_NORMAL);
bg_container!(BgMain,     BG_MAIN,    TEXT_NORMAL);
bg_container!(BgInput,    BG_INPUT,   TEXT_NORMAL);
bg_container!(BgHover,    BG_HOVER,   TEXT_NORMAL);
bg_container!(BgSelected, BG_SELECTED,TEXT_NORMAL);
bg_container!(BgModal,    BG_MODAL,   TEXT_NORMAL);
bg_container!(PanelLeft,  BG_DARK,    TEXT_NORMAL);
bg_container!(PanelMid,   BG_DARK,    TEXT_NORMAL);
bg_container!(PanelRight, BG_MAIN,    TEXT_NORMAL);
bg_container!(HoverRow,   BG_HOVER,   TEXT_NORMAL);

// ─── Specialized containers ───────────────────────────────────────────────────

pub struct CardStyle;
impl container::StyleSheet for CardStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(BG_MODAL)),
            border: colored_border(DIVIDER, 1.0, 12.0),
            text_color: Some(TEXT_NORMAL),
            shadow: shadow(24.0, 8.0, 0.45),
        }
    }
}

pub struct UnlockCardStyle;
impl container::StyleSheet for UnlockCardStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(BG_MODAL)),
            border: colored_border(Color { r:0.34, g:0.40, b:0.95, a:0.3 }, 1.0, 16.0),
            text_color: Some(TEXT_NORMAL),
            shadow: shadow(48.0, 12.0, 0.6),
        }
    }
}

pub struct MessageHeaderStyle;
impl container::StyleSheet for MessageHeaderStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color { r: 0.212, g: 0.224, b: 0.247, a: 0.98 })),
            border: colored_border(DIVIDER, 0.0, 0.0),
            text_color: Some(TEXT_WHITE),
            shadow: shadow(8.0, 4.0, 0.25),
        }
    }
}

pub struct ComposeBarStyle;
impl container::StyleSheet for ComposeBarStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(BG_MAIN)),
            border: colored_border(DIVIDER, 1.0, 0.0),
            text_color: Some(TEXT_NORMAL),
            shadow: Shadow::default(),
        }
    }
}

pub struct StatusBarStyle;
impl container::StyleSheet for StatusBarStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color { r:0.11, g:0.12, b:0.13, a:1.0 })),
            border: colored_border(DIVIDER, 1.0, 0.0),
            text_color: Some(TEXT_MUTED),
            shadow: Shadow::default(),
        }
    }
}

pub struct MemberCardStyle;
impl container::StyleSheet for MemberCardStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color { a: 0.4, ..BG_HOVER })),
            border: colored_border(Color { a:0.0, ..DIVIDER }, 0.0, 6.0),
            text_color: Some(TEXT_NORMAL),
            shadow: Shadow::default(),
        }
    }
}

pub struct NotifInfo;
impl container::StyleSheet for NotifInfo {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color { r:0.184, g:0.235, b:0.420, a:0.95 })),
            border: colored_border(BLURPLE, 1.0, 8.0),
            text_color: Some(TEXT_WHITE),
            shadow: shadow(12.0, 4.0, 0.4),
        }
    }
}

pub struct NotifSuccess;
impl container::StyleSheet for NotifSuccess {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color { r:0.10, g:0.24, b:0.16, a:0.95 })),
            border: colored_border(GREEN, 1.0, 8.0),
            text_color: Some(TEXT_WHITE),
            shadow: shadow(12.0, 4.0, 0.4),
        }
    }
}

pub struct NotifError;
impl container::StyleSheet for NotifError {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color { r:0.30, g:0.10, b:0.10, a:0.95 })),
            border: colored_border(RED, 1.0, 8.0),
            text_color: Some(TEXT_WHITE),
            shadow: shadow(12.0, 4.0, 0.4),
        }
    }
}

pub struct NotifWarn;
impl container::StyleSheet for NotifWarn {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color { r:0.28, g:0.21, b:0.05, a:0.95 })),
            border: colored_border(YELLOW, 1.0, 8.0),
            text_color: Some(TEXT_WHITE),
            shadow: shadow(12.0, 4.0, 0.4),
        }
    }
}

pub struct StaffChannelStyle;
impl container::StyleSheet for StaffChannelStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color { r:0.300, g:0.150, b:0.04, a:0.12 })),
            border: colored_border(Color { r:0.98, g:0.66, b:0.10, a:0.3 }, 1.0, 4.0),
            text_color: Some(YELLOW),
            shadow: Shadow::default(),
        }
    }
}

pub struct AnnouncementChannelStyle;
impl container::StyleSheet for AnnouncementChannelStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color { r:0.05, g:0.22, b:0.10, a:0.12 })),
            border: colored_border(Color { r:0.23, g:0.65, b:0.37, a:0.3 }, 1.0, 4.0),
            text_color: Some(GREEN),
            shadow: Shadow::default(),
        }
    }
}

pub struct InlineTagStyle { pub color: Color }
impl container::StyleSheet for InlineTagStyle {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color { a: 0.2, ..self.color })),
            border: colored_border(Color { a: 0.6, ..self.color }, 1.0, 4.0),
            text_color: Some(self.color),
            shadow: Shadow::default(),
        }
    }
}

pub struct AvatarContainer { pub color: Color, pub radius: f32 }
impl container::StyleSheet for AvatarContainer {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.color)),
            border: Border { color: Color::TRANSPARENT, width: 0.0, radius: self.radius.into() },
            text_color: Some(TEXT_WHITE),
            shadow: shadow(6.0, 2.0, 0.3),
        }
    }
}

pub struct RoleBadge { pub color: Color }
impl container::StyleSheet for RoleBadge {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color { a: 0.18, ..self.color })),
            border: colored_border(Color { a: 0.7, ..self.color }, 1.0, 4.0),
            text_color: Some(self.color),
            shadow: Shadow::default(),
        }
    }
}

pub struct UnreadBadge;
impl container::StyleSheet for UnreadBadge {
    type Style = Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(RED)),
            border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 8.0.into() },
            text_color: Some(TEXT_WHITE),
            shadow: Shadow::default(),
        }
    }
}

// ─── Button Styles ────────────────────────────────────────────────────────────

pub struct BlurpleButton;
impl button::StyleSheet for BlurpleButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BLURPLE)),
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            text_color: TEXT_WHITE,
            shadow: shadow(4.0, 2.0, 0.3),
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BLURPLE_DARK)),
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            text_color: TEXT_WHITE,
            shadow: shadow(6.0, 3.0, 0.4),
            ..Default::default()
        }
    }
    fn pressed(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color { r:0.22, g:0.26, b:0.62, a:1.0 })),
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
    fn disabled(&self, style: &Self::Style) -> button::Appearance {
        let mut a = self.active(style);
        a.background = Some(Background::Color(Color { a: 0.4, ..BLURPLE }));
        a
    }
}

pub struct DangerButton;
impl button::StyleSheet for DangerButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(RED)),
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            text_color: TEXT_WHITE,
            shadow: shadow(4.0, 2.0, 0.3),
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(RED_DARK)),
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            text_color: TEXT_WHITE,
            shadow: shadow(6.0, 3.0, 0.4),
            ..Default::default()
        }
    }
    fn pressed(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color { r:0.52, g:0.14, b:0.14, a:1.0 })),
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
}

pub struct SuccessButton;
impl button::StyleSheet for SuccessButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(GREEN)),
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            text_color: TEXT_WHITE,
            shadow: shadow(4.0, 2.0, 0.3),
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(GREEN_DARK)),
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            text_color: TEXT_WHITE,
            shadow: shadow(6.0, 3.0, 0.4),
            ..Default::default()
        }
    }
    fn pressed(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color { r:0.12, g:0.36, b:0.20, a:1.0 })),
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
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
            border: colored_border(BLURPLE, 1.0, 4.0),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
    fn pressed(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BG_SELECTED)),
            border: colored_border(BLURPLE, 1.5, 4.0),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
}

pub struct FlatButton;
impl button::StyleSheet for FlatButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border: no_border(),
            text_color: TEXT_MUTED,
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BG_HOVER)),
            border: rounded(4.0),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
    fn pressed(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BG_SELECTED)),
            border: rounded(4.0),
            text_color: TEXT_WHITE,
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
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        self.active(&Theme::Dark)
    }
    fn pressed(&self, _: &Self::Style) -> button::Appearance {
        self.active(&Theme::Dark)
    }
}

pub struct ServerIconButton;
impl button::StyleSheet for ServerIconButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border: no_border(),
            text_color: TEXT_NORMAL,
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border: no_border(),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
    fn pressed(&self, _: &Self::Style) -> button::Appearance {
        self.hovered(&Theme::Dark)
    }
}

pub struct ActiveServerIconButton;
impl button::StyleSheet for ActiveServerIconButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border: no_border(),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        self.active(&Theme::Dark)
    }
    fn pressed(&self, _: &Self::Style) -> button::Appearance {
        self.active(&Theme::Dark)
    }
}

pub struct IconButton { pub color: Color }
impl button::StyleSheet for IconButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border: no_border(),
            text_color: self.color,
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BG_HOVER)),
            border: rounded(4.0),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
    fn pressed(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(BG_SELECTED)),
            border: rounded(4.0),
            text_color: TEXT_WHITE,
            ..Default::default()
        }
    }
}

pub struct DestructiveFlatButton;
impl button::StyleSheet for DestructiveFlatButton {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border: no_border(),
            text_color: RED,
            ..Default::default()
        }
    }
    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color { a:0.15, ..RED })),
            border: rounded(4.0),
            text_color: RED,
            ..Default::default()
        }
    }
    fn pressed(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color { a:0.25, ..RED })),
            border: rounded(4.0),
            text_color: RED,
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
            border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 4.0.into() },
            icon_color: TEXT_MUTED,
        }
    }
    fn focused(&self, _: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(BG_INPUT),
            border: colored_border(BLURPLE, 2.0, 4.0),
            icon_color: BLURPLE,
        }
    }
    fn placeholder_color(&self, _: &Self::Style) -> Color { TEXT_MUTED }
    fn value_color(&self, _: &Self::Style) -> Color { TEXT_WHITE }
    fn disabled_color(&self, _: &Self::Style) -> Color { TEXT_MUTED }
    fn selection_color(&self, _: &Self::Style) -> Color {
        Color { r: 0.345, g: 0.396, b: 0.949, a: 0.3 }
    }
    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        let mut a = self.active(style);
        a.background = Background::Color(Color { a:0.5, ..BG_INPUT });
        a
    }
    fn hovered(&self, s: &Self::Style) -> text_input::Appearance {
        self.focused(s)
    }
}

// ─── Scrollable ───────────────────────────────────────────────────────────────

pub struct SlimScrollbar;
impl scrollable::StyleSheet for SlimScrollbar {
    type Style = Theme;
    fn active(&self, _: &Self::Style) -> scrollable::Appearance {
        let sb = scrollable::Scrollbar {
            background: None,
            border: no_border(),
            scroller: scrollable::Scroller {
                color: Color { r: 0.4, g: 0.4, b: 0.4, a: 0.4 },
                border: Border { radius: 3.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            },
        };
        scrollable::Appearance {
            container: container::Appearance::default(),
            scrollbar: sb,
            gap: None,
        }
    }
    fn hovered(&self, _: &Self::Style, _over_scrollbar: bool) -> scrollable::Appearance {
        let sb = scrollable::Scrollbar {
            background: None,
            border: no_border(),
            scroller: scrollable::Scroller {
                color: Color { r: 0.6, g: 0.6, b: 0.6, a: 0.6 },
                border: Border { radius: 3.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            },
        };
        scrollable::Appearance {
            container: container::Appearance::default(),
            scrollbar: sb,
            gap: None,
        }
    }
    fn dragging(&self, s: &Self::Style) -> scrollable::Appearance {
        self.hovered(s, true)
    }
}

// ─── Re-exports / aliases ─────────────────────────────────────────────────────

pub use BlurpleButton as AccentButton;
pub use BlurpleButton as SendButtonStyle;
pub use DiscordInput as DarkInputStyle;
pub use AvatarContainer as AvatarContainerStyle;
