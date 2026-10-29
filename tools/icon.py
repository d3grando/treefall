"""Draws icon.png, the registry icon: a tree falling from its stump, an axe leaning on the stump, in a vanilla
tooltip's frame (the frame LookAt's icon uses, so the mods look like one family). 32 x 32 pixel art scaled 8 times
to 256 x 256. Needs Pillow.

    python tools/icon.py [icon.png]
"""
import math
import sys

from PIL import Image

SIZE, SCALE = 32, 8
BACKGROUND = (0x10, 0x00, 0x10, 0xF0)
FRAME_TOP = (0x50, 0x00, 0xFF, 0xFF)
FRAME_BOTTOM = (0x28, 0x00, 0x7F, 0xFF)


def lerp(a, b, t):
    return tuple(round(ca + (cb - ca) * t) for ca, cb in zip(a, b))


class Canvas:
    """A 32 x 32 tooltip; drawing stays inside its frame."""

    def __init__(self):
        self.img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
        self.px = self.img.load()
        for y in range(SIZE):
            for x in range(SIZE):
                if x in (0, SIZE - 1) and y in (0, SIZE - 1):
                    continue
                edge = x in (0, SIZE - 1) or y in (0, SIZE - 1)
                frame = x in (1, SIZE - 2) or y in (1, SIZE - 2)
                self.px[x, y] = lerp(FRAME_TOP, FRAME_BOTTOM, (y - 1) / (SIZE - 3)) if frame and not edge else BACKGROUND

    def set(self, x, y, c):
        if not (2 <= x < SIZE - 2 and 2 <= y < SIZE - 2) or c is None:
            return
        if len(c) == 4 and c[3] < 255:
            under, a = self.px[x, y], c[3] / 255
            c = tuple(round(u + (v - u) * a) for u, v in zip(under[:3], c[:3])) + (under[3],)
        self.px[x, y] = c if len(c) == 4 else c + (255,)

    def sprite(self, x0, y0, art, palette):
        for dy, row in enumerate(art):
            for dx, ch in enumerate(row):
                if ch not in ". ":
                    self.set(x0 + dx, y0 + dy, palette[ch])

    def save(self, path):
        self.img.resize((SIZE * SCALE, SIZE * SCALE), Image.NEAREST).save(path, optimize=True)


PALETTE = {
    # leaves: shadow, body, light
    "L": (0x1F, 0x5E, 0x24), "l": (0x3A, 0x8E, 0x30), "g": (0x6C, 0xC4, 0x48),
    # bark: shadow, light; the cut wood: rings
    "T": (0x4E, 0x30, 0x1A), "t": (0x7E, 0x52, 0x30), "R": (0xD8, 0xB0, 0x70), "r": (0xA8, 0x7C, 0x48),
    # the axe: steel, its shadow, the edge; the handle
    "S": (0xC8, 0xCC, 0xD4), "s": (0x7C, 0x80, 0x8C), "E": (0xFF, 0xFF, 0xFF), "H": (0xA8, 0x74, 0x40),
    "h": (0x6A, 0x44, 0x22),
    # the ground; the motion lines of the fall
    "G": (0x4C, 0x9A, 0x34), "D": (0x6A, 0x46, 0x2A), "w": (0xFF, 0xFF, 0xFF, 0x90),
}

# The stump, its cut face on top, at (11, 20).
STUMP = [
    ".rRRRRr.",
    "rRRrrRRr",
    "tRRRRRRT",
    "ttRRRRTT",
    "tttTTTTT",
    "tttTTTTT",
    "ttTtTTTT",
]


def blob(c, discs):
    """Leaves filling the union of `discs` (cx, cy, r), lit from the top left, with a few lighter and darker spots."""
    for y in range(SIZE):
        for x in range(SIZE):
            hits = [(cx, cy, r) for cx, cy, r in discs if (x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2 <= r * r]
            if not hits:
                continue
            cx, cy, r = discs[0]
            n = (x + 0.5 - cx) + (y + 0.5 - cy)
            colour = "g" if n < -0.75 * r else "L" if n > 0.45 * r else "l"
            if (x * 7 + y * 5) % 13 == 0:
                colour = {"g": "l", "l": "g", "L": "l"}[colour]
            elif (x * 3 + y * 11) % 17 == 0 and colour == "l":
                colour = "L"
            c.set(x, y, PALETTE[colour])


def trunk(c, a, b, half):
    """A log from `a` to `b`, `half` pixels either side of its axis, lit on its upper side."""
    (ax, ay), (bx, by) = a, b
    dx, dy = bx - ax, by - ay
    length2 = dx * dx + dy * dy
    for y in range(SIZE):
        for x in range(SIZE):
            px, py = x + 0.5 - ax, y + 0.5 - ay
            t = max(0.0, min(1.0, (px * dx + py * dy) / length2))
            ex, ey = px - t * dx, py - t * dy
            if ex * ex + ey * ey <= half * half:
                side = ex * dy - ey * dx  # < 0: the upper left side
                c.set(x, y, PALETTE["t" if side < -0.2 * (length2 ** 0.5) else "T"])


def axe(c, tx, ty, along, blade, length):
    """An axe leaning on something: the handle from (tx, ty) in the diagonal `along`, the head at (tx, ty), its blade
    facing the diagonal `blade`."""
    for i in range(length):
        c.set(tx + i * along[0], ty + i * along[1], PALETTE["H"])
        c.set(tx + i * along[0], ty + i * along[1] + 1, PALETTE["h"])
    for dy in range(-10, 11):
        for dx in range(-10, 11):
            v = (dx * along[0] + dy * along[1])  # along the handle, its end > 0
            u = (dx * blade[0] + dy * blade[1])  # across it, the blade > 0
            if not -2 <= u <= 7:
                continue
            w = 2 if u < 3 else 2 + (u - 2) // 2
            if not -w - 1 <= v <= w - 1:
                continue
            if u >= 6:
                colour = "E"
            elif u <= -1 or v >= w - 2:
                colour = "s"
            else:
                colour = "S"
            c.set(tx + dx, ty + dy, PALETTE[colour])


def draw():
    c = Canvas()
    # the ground
    for x in range(3, 29):
        c.set(x, 27, PALETTE["G"])
        c.set(x, 28, PALETTE["D"])
    # the motion lines of the fall: arcs round the stump, where the crown has just been
    for radius, (start, end) in ((11, (-118, -82)), (14, (-108, -78))):
        for step in range(start, end, 4):
            a = math.radians(step)
            c.set(round(16.5 + radius * math.cos(a)), round(19.5 + radius * math.sin(a)), PALETTE["w"])
    c.sprite(11, 20, STUMP, PALETTE)
    trunk(c, (16.5, 19.0), (22.0, 14.0), 1.6)
    blob(c, [(23.0, 10.0, 5.4), (19.0, 8.0, 3.0), (25.0, 15.0, 2.8), (24.5, 5.6, 2.6)])
    axe(c, 9, 20, (-1, 1), (-1, -1), 7)
    return c


if __name__ == "__main__":
    draw().save(sys.argv[1] if len(sys.argv) > 1 else "icon.png")
