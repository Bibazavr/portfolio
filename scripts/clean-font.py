# Prepares the plaza's font: removes overlapping contours, which meshtext's
# triangulation can't handle (Rubik's static instances keep them, and glyphs
# like "и" and "у" then fail), and keeps only the Latin and Cyrillic the
# content uses, which makes the WASM smaller. Hinting and OpenType layout go
# too: meshtext uses neither.
#
# Needs fonttools and skia-pathops:
#
#   pip install fonttools skia-pathops
#   python3 scripts/clean-font.py Rubik-Bold.ttf crates/plaza/assets/fonts/Rubik-Bold.ttf
#
# Run it on the font as downloaded from Google Fonts, not on its own output.

import sys

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.ttLib.removeOverlaps import removeOverlaps

KEEP = [
    *range(0x0020, 0x0250),  # Basic Latin to Latin Extended-B
    *range(0x0400, 0x0530),  # Cyrillic and Cyrillic Supplement
    *range(0x2010, 0x2040),  # Dashes, quotes, bullets, ellipsis
    0x20AC,  # €
    0x20BD,  # ₽
    0x20B8,  # ₸
    0x2116,  # №
]

source, target = sys.argv[1:]
font = TTFont(source)
removeOverlaps(font)
options = subset.Options()
options.layout_features = []
options.hinting = False
options.name_IDs = ["*"]
subsetter = subset.Subsetter(options)
subsetter.populate(unicodes=KEEP)
subsetter.subset(font)
font.save(target)
