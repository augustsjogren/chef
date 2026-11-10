#!/usr/bin/env python3
"""Render tmux `capture-pane -e` ANSI text to PNG (monospace grid, truecolor)."""
import sys
import re
from PIL import Image, ImageDraw, ImageFont

COLS, ROWS = 132, 38
CELL_W, CELL_H = 10, 17
BG = (27, 31, 39)
FG = (206, 212, 222)
FG16 = {30: (90, 96, 106), 31: (205, 92, 80), 32: (138, 168, 122),
        33: (232, 180, 90), 34: (110, 140, 180), 35: (180, 140, 200),
        36: (120, 170, 180), 37: FG, 90: (120, 126, 134),
        91: (215, 110, 95), 92: (160, 195, 140), 93: (245, 200, 120),
        94: (140, 170, 210), 95: (200, 160, 220), 96: (150, 200, 210), 97: FG}
ANSI_256 = [(224, 228, 236)] * 256

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
    font = ImageFont.truetype("/System/Library/Fonts/SFNSMono.ttf", 13)
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
