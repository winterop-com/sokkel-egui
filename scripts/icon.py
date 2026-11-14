# /// script
# requires-python = ">=3.10"
# dependencies = ["pillow"]
# ///
"""Render a macOS app icon (.icns) for one example app.

A rounded square (Big Sur grid: 824 px body on a 1024 px canvas) filled with a diagonal gradient
from the base theme's primary color (crates/core/tokens/tokens.resolved.json) to a darker shade of
it, with the app's initial in IBM Plex Sans, the library's face, in the primary's foreground. Writes
an .iconset with every size iconutil wants, then runs `iconutil -c icns`. Used by
scripts/bundle.sh.

Run: uv run scripts/icon.py <Title> <out.icns>
"""

import json
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
FONT = ROOT / "crates" / "core" / "assets" / "fonts" / "IBMPlexSans-Variable-Latin.ttf"
TOKENS = ROOT / "crates" / "core" / "tokens" / "tokens.resolved.json"
S = 1024


def rgb(h: str) -> tuple[int, int, int]:
    return tuple(int(h[i : i + 2], 16) for i in (1, 3, 5))


def colors() -> tuple[tuple[int, int, int], tuple[int, int, int], tuple[int, int, int]]:
    """The base theme's primary, a darker shade of it, and the primary's foreground."""
    light = json.loads(TOKENS.read_text())["themes"]["tidevann"]["light"]["colors"]
    primary = rgb(light["primary"][:7])
    ink = rgb(light["primary-foreground"][:7])
    dark = tuple(int(c * 0.55) for c in primary)
    return primary, dark, ink


def render(title: str) -> Image.Image:
    top, bottom, ink = colors()
    # Diagonal gradient, top-left -> bottom-right: the average of a vertical and a horizontal ramp.
    v = Image.linear_gradient("L").resize((S, S))
    ramp = ImageChops.add(v, v.transpose(Image.Transpose.TRANSPOSE), scale=2)
    grad = Image.composite(Image.new("RGB", (S, S), bottom), Image.new("RGB", (S, S), top), ramp)

    body, radius = 824, 185
    off = (S - body) // 2
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).rounded_rectangle((off, off, off + body, off + body), radius, fill=255)
    icon = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    icon.paste(grad, (0, 0), mask)

    d = ImageDraw.Draw(icon)
    # A hairline highlight around the edge instead of a drop shadow.
    d.rounded_rectangle((off, off, off + body, off + body), radius, outline=(255, 255, 255, 46), width=6)

    text = title[:1].upper()
    font = ImageFont.truetype(str(FONT), 460)
    try:
        # every axis at its default but the weight, at 600
        axes = font.get_variation_axes()
        weight = lambda axis: str(axis.get("name", "")).lower().find("weight") >= 0
        font.set_variation_by_axes([600 if weight(a) else a["default"] for a in axes])
    except (OSError, AttributeError, ValueError):
        pass
    box = d.textbbox((0, 0), text, font=font, anchor="lt")
    w, h = box[2] - box[0], box[3] - box[1]
    x, y = (S - w) / 2 - box[0], (S - h) / 2 - box[1]
    shadow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).text((x, y + 10), text, font=font, fill=(0, 0, 0, 70))
    icon = Image.alpha_composite(icon, shadow)
    ImageDraw.Draw(icon).text((x, y), text, font=font, fill=(*ink, 245))
    # Clip the shadow to the body.
    icon.putalpha(ImageChops.multiply(icon.getchannel("A"), mask))
    return icon


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit("usage: icon.py <Title> <out.icns>")
    title, out = sys.argv[1], Path(sys.argv[2])
    icon = render(title)
    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp) / "icon.iconset"
        iconset.mkdir()
        for size in (16, 32, 128, 256, 512):
            icon.resize((size, size), Image.LANCZOS).save(iconset / f"icon_{size}x{size}.png")
            icon.resize((size * 2, size * 2), Image.LANCZOS).save(iconset / f"icon_{size}x{size}@2x.png")
        out.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(out)], check=True)


if __name__ == "__main__":
    main()
