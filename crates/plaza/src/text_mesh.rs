//! Text as meshes, for the words painted, printed and standing around the
//! plaza. Glyphs come from Rubik Bold (SIL OFL, see assets/fonts/OFL-Rubik.txt),
//! which covers Latin and Cyrillic, through `meshtext`. Rubik has no arrows.

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use meshtext::{Face, Glyph, MeshGenerator, MeshText, TextSection};

const FONT: &[u8] = include_bytes!("../assets/fonts/Rubik-Bold.ttf");
/// Rubik's capitals are this tall, in ems.
const CAP_HEIGHT: f32 = 0.59;
/// Distance between baselines, in cap heights.
const LINE_PITCH: f32 = 1.9;

/// The font, with the glyph meshes it has made so far.
#[derive(Resource)]
pub struct Font(MeshGenerator<Face<'static>>);

impl Default for Font {
    fn default() -> Self {
        Self(MeshGenerator::new(FONT))
    }
}

/// How a piece of text is set. Lengths are in metres.
#[derive(Clone, Copy)]
pub struct Style {
    /// The height of a capital letter.
    pub size: f32,
    /// Lines wrap at word boundaries to stay within this.
    pub max_width: f32,
    /// The text gets smaller until its lines fit in this.
    pub max_height: f32,
    /// One line, made smaller rather than wrapped when it is too wide.
    fit: bool,
    /// Centred on y = 0 rather than hanging from it.
    middle: bool,
}

impl Style {
    pub const fn new(size: f32) -> Self {
        Self {
            size,
            max_width: f32::INFINITY,
            max_height: f32::INFINITY,
            fit: false,
            middle: false,
        }
    }

    pub const fn width(mut self, max_width: f32) -> Self {
        self.max_width = max_width;
        self
    }

    pub const fn height(mut self, max_height: f32) -> Self {
        self.max_height = max_height;
        self
    }

    pub const fn fit(mut self) -> Self {
        self.fit = true;
        self
    }

    pub const fn middle(mut self) -> Self {
        self.middle = true;
        self
    }
}

impl Font {
    /// Flat text in the XY plane, facing +Z: lines centred on x = 0, the
    /// first line's capitals hanging from y = 0 (or the block centred on it).
    pub fn flat(&mut self, text: &str, style: Style) -> Mesh {
        let mut size = style.size;
        let height = |lines: usize, size: f32| size + (lines - 1) as f32 * LINE_PITCH * size;
        let lines = if style.fit {
            let width = self.width(text) * size / CAP_HEIGHT;
            if width > style.max_width {
                size *= style.max_width / width;
            }
            vec![text.to_owned()]
        } else {
            let mut lines = self.wrap(text, size, style.max_width);
            while height(lines.len(), size) > style.max_height && size > style.size / 2.0 {
                size *= 0.95;
                lines = self.wrap(text, size, style.max_width);
            }
            lines
        };
        let em = size / CAP_HEIGHT;
        let top = if style.middle {
            height(lines.len(), size) / 2.0
        } else {
            0.0
        };

        let mut positions = Vec::new();
        for (row, line) in lines.iter().enumerate() {
            let Ok(mesh) = TextSection::<MeshText>::generate_section(&mut self.0, line, true, None)
            else {
                continue;
            };
            let x = -mesh.bbox.max.x * em / 2.0;
            let baseline = top - size - row as f32 * LINE_PITCH * size;
            positions.extend(
                mesh.vertices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|[vx, vy, _]| [x + vx * em, baseline + vy * em, 0.0]),
            );
        }
        let normals = vec![[0.0, 0.0, 1.0]; positions.len()];
        let indices = Indices::U32((0..positions.len() as u32).collect());
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_indices(indices)
    }

    /// One solid letter with capitals `size` tall, `depth` thick and centred
    /// on the origin, with the size of its box. `None` for a space.
    pub fn letter(&mut self, letter: char, size: f32, depth: f32) -> Option<(Mesh, Vec3)> {
        let mesh = Glyph::<MeshText>::generate_glyph(&mut self.0, letter, false, None).ok()?;
        // meshtext has its own glam.
        let (min, max) = (mesh.bbox.min.to_array(), mesh.bbox.max.to_array());
        let (min, max) = (Vec3::from_array(min), Vec3::from_array(max));
        if mesh.vertices.is_empty() || max.x <= min.x {
            return None;
        }
        let center = (min + max) / 2.0;
        let em = size / CAP_HEIGHT;
        let scale = Vec3::new(em, em, depth / (max.z - min.z));
        let positions: Vec<[f32; 3]> = mesh
            .vertices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|&v| ((Vec3::from_array(v) - center) * scale).to_array())
            .collect();
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.compute_flat_normals();
        Some((mesh, (max - min) * scale))
    }

    /// Breaks `text` set at `size` into lines no wider than `max_width`.
    /// Line breaks in the text are kept.
    fn wrap(&mut self, text: &str, size: f32, max_width: f32) -> Vec<String> {
        let max_width = max_width * CAP_HEIGHT / size;
        let mut lines = Vec::new();
        for paragraph in text.lines() {
            let mut line = String::new();
            for word in paragraph.split_whitespace() {
                let candidate = if line.is_empty() {
                    word.to_owned()
                } else {
                    format!("{line} {word}")
                };
                if !line.is_empty() && self.width(&candidate) > max_width {
                    lines.push(std::mem::replace(&mut line, word.to_owned()));
                } else {
                    line = candidate;
                }
            }
            lines.push(line);
        }
        lines
    }

    /// Width in ems.
    fn width(&mut self, text: &str) -> f32 {
        TextSection::<MeshText>::generate_section(&mut self.0, text, true, None)
            .map_or(0.0, |mesh| mesh.bbox.max.x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every character in the content has a glyph that meshtext can build.
    /// It can't when a glyph's contours overlap (see scripts/clean-font.py).
    #[test]
    fn content_characters_render() {
        // Only in project points, which the plaza doesn't show; Rubik has no arrows.
        const NOT_IN_PLAZA: &[char] = &['→'];
        let mut font = Font::default();
        let mut characters: Vec<char> = content::SOURCE
            .chars()
            .filter(|c| !c.is_whitespace() && !NOT_IN_PLAZA.contains(c))
            .collect();
        characters.sort_unstable();
        characters.dedup();
        let missing: String = characters
            .into_iter()
            .filter(|&c| {
                TextSection::<MeshText>::generate_section(&mut font.0, &c.to_string(), true, None)
                    .is_ok_and(|mesh| mesh.vertices.is_empty())
                    || Glyph::<MeshText>::generate_glyph(&mut font.0, c, true, None).is_err()
            })
            .collect();
        assert!(missing.is_empty(), "no glyph for {missing:?}");
    }
}
