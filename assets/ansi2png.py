#!/usr/bin/env python3
"""Render tmux `capture-pane -e` ANSI text to PNG (monospace grid, truecolor)."""
import sys
import re
from PIL import Image, ImageDraw, ImageFont

COLS, ROWS = 132, 38
CELL_W, CELL_H = 12, 20
# Ghostty everforest-dark (matches the author terminal theme)
BG = (45, 53, 59)      # #2d353b
FG = (211, 198, 170)   # #d3c6aa
RED = (230, 126, 128)  # #e67e80
GREEN = (167, 192, 128)  # #a7c080
YELLOW = (219, 188, 127)  # #dbbc7f
BLUE = (127, 187, 179)   # #7fbbb3
MAGENTA = (214, 153, 182)  # #d699b6
CYAN = (131, 192, 146)   # #83c092
MUTED = (133, 146, 137)  # #859289
FG16 = {30: MUTED, 31: RED, 32: GREEN, 33: YELLOW, 34: BLUE, 35: MAGENTA,
        36: CYAN, 37: FG, 90: MUTED, 91: RED, 92: GREEN, 93: YELLOW,
        94: BLUE, 95: MAGENTA, 96: CYAN, 97: FG}
ANSI_256 = [FG] * 256

CSI = re.compile(r"\x1b\[([0-9;:]*)([a-zA-Z])")


def render(text, out):
    fg, bg, bold, reverse = FG, None, False, False
    r, c, i = 0, 0, 0
    grid = {}
    while i < len(text):
        ch = text[i]
        if ch == "\x1b":
            m = CSI.match(text, i)
            if m:
                params, final = m.group(1), m.group(2)
                if final == "m":
                    ps = [p if p else "0" for p in params.split(";")]
                    if ps[-1] == "0":
                        fg, bg, bold, reverse = FG, None, False, False
                    k = 0
                    while k < len(ps):
                        p = ps[k]
                        if p == "0":
                            fg, bg, bold, reverse = FG, None, False, False
                        elif p == "1":
                            bold = True
                        elif p == "7":
                            reverse = True
                        elif p in ("22", "23", "27"):
                            bold, reverse = False, False
                        elif p == "39":
                            fg = FG
                        elif p == "49":
                            bg = None
                        elif len(p) == 2 and p.lstrip("0123456789") == "":
                            v = int(p)
                            if 30 <= v <= 37 or 90 <= v <= 97:
                                fg = FG16[v]
                        elif p == "38" and k + 1 < len(ps) and ps[k + 1] == "2" and k + 4 < len(ps):
                            fg = (int(ps[k + 2]), int(ps[k + 3]), int(ps[k + 4]))
                            k += 4
                        k += 1
                i = m.end()
                continue
            i += 1
            continue
        if ch == "\r":
            c = 0
        elif ch == "\n":
            r += 1
            c = 0
        elif ord(ch) < 32:
            pass
        elif r < ROWS and c < COLS:
            if reverse:
                # terminal swaps fg/bg: bar in fg color, text in dark bg color
                grid[(r, c)] = (ch, BG, fg if bg is None else bg)
            else:
                grid[(r, c)] = (ch, fg, bg)
            c += 1
        else:
            c += 1
        i += 1

    img = Image.new("RGB", (COLS * CELL_W, ROWS * CELL_H), BG)
    draw = ImageDraw.Draw(img)
    font = ImageFont.truetype(
        "/Users/augustsjogren/Library/Fonts/FiraCodeNerdFontMono-Regular.ttf", 14)
    for (row, col), (gch, cng, cbg) in grid.items():
        x, y = col * CELL_W, row * CELL_H
        if cbg:
            draw.rectangle([x, y, x + CELL_W - 1, y + CELL_H - 1], fill=cbg)
        try:
            draw.text((x + 1, y), gch, fill=cng, font=font)
        except Exception:
            pass
    img.save(out)
    print("saved", out)


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit("usage: ansi2png.py in.ansi out.png")
    with open(sys.argv[1], encoding="utf-8") as fh:
        render(fh.read(), sys.argv[2])
