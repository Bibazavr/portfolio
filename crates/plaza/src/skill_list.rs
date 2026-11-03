//! The skills, laid out on the ground in front of the skill shop: a block per
//! group, a heading over its list, in balanced columns that read down and
//! then across, in the content's order. Each block is sized to fit its group
//! in either language, so the layout is made once and only the words change
//! with the language.

use std::ops::Range;

use content::{Lang, SkillGroup, Text};

use crate::{
    signs,
    text_mesh::{Font, Style},
};

/// A column, the gap between columns, and the gap between blocks.
pub const COLUMN: f32 = 3.2;
const GAP: f32 = 0.35;
/// Between a block's heading and its list.
const UNDER_HEADING: f32 = 0.16;
const MAX_COLUMNS: usize = 8;

pub fn heading_style() -> Style {
    Style::new(0.19).width(COLUMN).left()
}

pub fn list_style() -> Style {
    Style::new(0.12).width(COLUMN).left()
}

/// One group's heading and list.
pub struct Block {
    pub heading: Text,
    pub list: Text,
    /// The middle of the block's top, from the layout's top left corner (x
    /// to the right, y down).
    pub top: (f32, f32),
    /// Where the list starts, below the block's top.
    pub list_below: f32,
    height: f32,
}

/// The blocks, and the size of the layout.
pub struct Layout {
    pub blocks: Vec<Block>,
    pub width: f32,
    pub height: f32,
}

/// Lays `groups` out in as few columns as keeps them within `max_height`.
pub fn lay_out(font: &mut Font, groups: &[SkillGroup], max_height: f32) -> Layout {
    let mut blocks: Vec<Block> = groups
        .iter()
        .map(|group| {
            let heading = signs::each(|lang| group.name.get(lang).to_uppercase());
            let list = signs::each(|lang| {
                let items: Vec<_> = group
                    .items
                    .iter()
                    .map(|item| format!("– {}", item.get(lang)))
                    .collect();
                items.join("\n")
            });
            let mut measure = |text: &Text, style: Style| {
                Lang::ALL
                    .map(|lang| font.height(text.get(lang), style))
                    .into_iter()
                    .fold(0.0, f32::max)
            };
            let list_below = measure(&heading, heading_style()) + UNDER_HEADING;
            let height = list_below + measure(&list, list_style());
            Block {
                heading,
                list,
                top: (0.0, 0.0),
                list_below,
                height,
            }
        })
        .collect();

    let heights: Vec<f32> = blocks.iter().map(|block| block.height).collect();
    let columns = (1..=MAX_COLUMNS)
        .map(|count| split(&heights, count))
        .find(|columns| tallest(&heights, columns) <= max_height)
        .unwrap_or_else(|| split(&heights, MAX_COLUMNS));
    for (column, range) in columns.iter().enumerate() {
        let x = column as f32 * (COLUMN + GAP) + COLUMN / 2.0;
        let mut y = 0.0;
        for block in &mut blocks[range.clone()] {
            block.top = (x, y);
            y += block.height + GAP;
        }
    }
    Layout {
        width: columns.len() as f32 * (COLUMN + GAP) - GAP,
        height: tallest(&heights, &columns),
        blocks,
    }
}

/// Splits blocks `heights` tall, in order, into at most `count` columns, as
/// evenly as it can.
fn split(heights: &[f32], count: usize) -> Vec<Range<usize>> {
    // The columns made by filling each up to `limit` before starting the next.
    let fill = |limit: f32| {
        let mut starts = vec![0];
        let mut height = 0.0;
        for (i, &h) in heights.iter().enumerate() {
            if i > 0 && height + GAP + h > limit {
                starts.push(i);
                height = h;
            } else {
                height += if i == 0 { h } else { GAP + h };
            }
        }
        let ends = starts.iter().skip(1).copied().chain([heights.len()]);
        let columns: Vec<Range<usize>> = starts.iter().zip(ends).map(|(&s, e)| s..e).collect();
        columns
    };
    // The best limit is between the tallest block and all of them stacked.
    let mut low = heights.iter().copied().fold(0.0, f32::max);
    let mut high = heights.iter().map(|h| h + GAP).sum::<f32>();
    for _ in 0..40 {
        let middle = (low + high) / 2.0;
        if fill(middle).len() <= count {
            high = middle;
        } else {
            low = middle;
        }
    }
    fill(high)
}

/// The tallest of `columns` of blocks `heights` tall.
fn tallest(heights: &[f32], columns: &[Range<usize>]) -> f32 {
    columns
        .iter()
        .map(|range| {
            let blocks = &heights[range.clone()];
            blocks.iter().sum::<f32>() + GAP * blocks.len().saturating_sub(1) as f32
        })
        .fold(0.0, f32::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Columns keep the blocks in order, one after another, and are as
    /// even as they can be.
    #[test]
    fn split_keeps_order() {
        let heights = [1.0, 2.0, 0.5, 0.5, 3.0, 1.0];
        let columns = split(&heights, 3);
        assert!(columns.len() <= 3);
        assert_eq!(columns.first().unwrap().start, 0);
        assert_eq!(columns.last().unwrap().end, heights.len());
        for pair in columns.windows(2) {
            assert_eq!(pair[0].end, pair[1].start);
        }
        // The best: 1, 2 and 0.5; 0.5 and 3; then 1, with gaps of 0.35.
        assert!(tallest(&heights, &columns) <= 4.2 + 1e-3);
    }
}
