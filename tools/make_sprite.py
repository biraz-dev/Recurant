#!/usr/bin/env python3
"""Turn a large reference image on a black background into a game sprite.

    python3 tools/make_sprite.py art/zombie-reference.png crates/zrpg/assets/zombie.png --height 64

Steps: the black background is removed by flood fill from the image border
(so dark details *inside* the figure, like eye sockets, stay opaque), narrow
gaps such as those between ribs are closed so the body reads as solid, the
figure is cropped, area-averaged down to `--height` pixels, its alpha is
made binary (the game uses alpha masking), and its colours are reduced to a
small palette for a crisp pixel-art look, with a one-pixel dark outline so
the figure stands out against the walls.

Needs Pillow and NumPy.
"""
import argparse
from collections import deque

import numpy as np
from PIL import Image


def background_mask(rgb: np.ndarray, threshold: int) -> np.ndarray:
    """True where a near-black pixel is connected to the image border."""
    h, w, _ = rgb.shape
    dark = rgb.max(axis=2) <= threshold
    bg = np.zeros((h, w), dtype=bool)
    queue = deque()
    for x in range(w):
        for y in (0, h - 1):
            if dark[y, x] and not bg[y, x]:
                bg[y, x] = True
                queue.append((y, x))
    for y in range(h):
        for x in (0, w - 1):
            if dark[y, x] and not bg[y, x]:
                bg[y, x] = True
                queue.append((y, x))
    while queue:
        y, x = queue.popleft()
        for ny, nx in ((y - 1, x), (y + 1, x), (y, x - 1), (y, x + 1)):
            if 0 <= ny < h and 0 <= nx < w and dark[ny, nx] and not bg[ny, nx]:
                bg[ny, nx] = True
                queue.append((ny, nx))
    return bg


def _max_filter(mask: np.ndarray, radius: int, axis: int) -> np.ndarray:
    out = mask.copy()
    for shift in range(1, radius + 1):
        out[tuple(slice(shift, None) if a == axis else slice(None) for a in range(2))] |= mask[
            tuple(slice(None, -shift) if a == axis else slice(None) for a in range(2))
        ]
        out[tuple(slice(None, -shift) if a == axis else slice(None) for a in range(2))] |= mask[
            tuple(slice(shift, None) if a == axis else slice(None) for a in range(2))
        ]
    return out


def close_gaps(mask: np.ndarray, radius: int) -> np.ndarray:
    """Morphological closing with a square of side 2*radius+1: fills gaps
    narrower than about 2*radius pixels, leaves wider ones alone."""
    if radius <= 0:
        return mask
    grown = _max_filter(_max_filter(mask, radius, 0), radius, 1)
    return ~_max_filter(_max_filter(~grown, radius, 0), radius, 1)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("src")
    ap.add_argument("dst")
    ap.add_argument("--height", type=int, default=64, help="sprite height in pixels")
    ap.add_argument("--colors", type=int, default=24, help="palette size")
    ap.add_argument("--threshold", type=int, default=24, help="max channel value counted as background")
    ap.add_argument("--no-outline", action="store_true", help="skip the dark outline")
    ap.add_argument("--close", type=int, default=10, help="fill gaps narrower than about twice this (source pixels)")
    args = ap.parse_args()

    rgb = np.asarray(Image.open(args.src).convert("RGB"))
    alpha = close_gaps(~background_mask(rgb, args.threshold), args.close)
    ys, xs = np.nonzero(alpha)
    y0, y1, x0, x1 = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
    rgb, alpha = rgb[y0:y1, x0:x1], alpha[y0:y1, x0:x1]

    h = args.height
    w = max(1, round(h * (x1 - x0) / (y1 - y0)))
    # Area-average colour weighted by coverage, so the dark background
    # doesn't bleed into the edges.
    a = alpha.astype(np.float32)
    premul = Image.fromarray((rgb * a[..., None]).astype(np.uint8))
    cover = Image.fromarray((a * 255).astype(np.uint8))
    premul = np.asarray(premul.resize((w, h), Image.Resampling.BOX)).astype(np.float32)
    cover = np.asarray(cover.resize((w, h), Image.Resampling.BOX)).astype(np.float32) / 255
    colour = premul / np.maximum(cover[..., None], 1e-6)
    opaque = cover >= 0.5

    small = Image.fromarray(np.clip(colour, 0, 255).astype(np.uint8))
    palette = small.quantize(colors=args.colors, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    out = np.asarray(palette.convert("RGB"))
    rgba = np.dstack([out, np.where(opaque, 255, 0).astype(np.uint8)])
    rgba[~opaque, :3] = 0
    if not args.no_outline:
        rgba = np.pad(rgba, ((1, 1), (1, 1), (0, 0)))
        solid = rgba[..., 3] > 0
        edge = np.zeros_like(solid)
        edge[1:, :] |= solid[:-1, :]
        edge[:-1, :] |= solid[1:, :]
        edge[:, 1:] |= solid[:, :-1]
        edge[:, :-1] |= solid[:, 1:]
        edge &= ~solid
        rgba[edge] = (18, 12, 10, 255)
    Image.fromarray(rgba, "RGBA").save(args.dst, optimize=True)
    print(f"{args.dst}: {rgba.shape[1]}x{rgba.shape[0]}, {args.colors} colours")


if __name__ == "__main__":
    main()
