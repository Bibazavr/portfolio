//! Colours: the studio site's accent and ink, plus the plaza's own surfaces.

use bevy::color::Color;

pub const ACCENT: Color = Color::srgb_u8(0x78, 0x51, 0xa9);
pub const INK: Color = Color::srgb_u8(0x14, 0x17, 0x1a);
/// The dark theme's accent, for a lit link pad.
pub const ACCENT_LIGHT: Color = Color::srgb_u8(0xb7, 0x96, 0xf0);
pub const PAPER: Color = Color::srgb_u8(0xfa, 0xfa, 0xf7);
/// Also the shell page's background, so there's no flash while loading.
pub const SKY: Color = Color::srgb_u8(0xdd, 0xe3, 0xe8);
pub const CONCRETE: Color = Color::srgb_u8(0xc4, 0xc7, 0xbf);
pub const CONCRETE_DARK: Color = Color::srgb_u8(0x9a, 0x9e, 0x96);
pub const METAL: Color = Color::srgb_u8(0x3a, 0x3f, 0x44);
