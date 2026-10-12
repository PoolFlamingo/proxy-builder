"""Genera el reverso genérico de Proxy Builder (745×1040, como el PNG de Scryfall).

Diseño propio y original: borde negro como el de una carta, marco dorado doble
y un emblema geométrico en el centro. Sin textos ni marcas de terceros (nunca el
reverso de Wizards). Se ejecuta a mano cuando cambie el diseño:

    python3 scripts/reverso.py
"""
import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

W, H = 745, 1040
BORDE = 34  # ~3 mm de borde negro, como una carta real
img = Image.new("RGB", (W, H), (8, 8, 8))

# Fondo: degradado radial cálido dentro del borde.
fondo = Image.new("RGB", (W - 2 * BORDE, H - 2 * BORDE))
fw, fh = fondo.size
px = fondo.load()
cx, cy = fw / 2, fh / 2
rmax = math.hypot(cx, cy)
for y in range(fh):
    for x in range(fw):
        t = math.hypot(x - cx, (y - cy) * 0.85) / rmax
        t = min(1.0, t)
        a = (58, 40, 22)
        b = (14, 11, 9)
        px[x, y] = tuple(int(a[i] * (1 - t) + b[i] * t) for i in range(3))
img.paste(fondo, (BORDE, BORDE))

d = ImageDraw.Draw(img)
oro = (205, 162, 74)
oro_osc = (128, 96, 40)
m = BORDE + 18
d.rounded_rectangle((m, m, W - m, H - m), radius=26, outline=oro, width=6)
m2 = m + 16
d.rounded_rectangle((m2, m2, W - m2, H - m2), radius=18, outline=oro_osc, width=2)

# Emblema: estrella de ocho puntas en un óvalo.
ecx, ecy = W / 2, H / 2
d.ellipse((ecx - 210, ecy - 290, ecx + 210, ecy + 290), outline=oro, width=5)
d.ellipse((ecx - 186, ecy - 262, ecx + 186, ecy + 262), outline=oro_osc, width=2)


def estrella(r_ext, r_int, puntas, rot, fill, outline=None):
    pts = []
    for i in range(puntas * 2):
        r = r_ext if i % 2 == 0 else r_int
        a = rot + math.pi * i / puntas
        pts.append((ecx + r * math.sin(a), ecy - r * math.cos(a)))
    d.polygon(pts, fill=fill, outline=outline)


estrella(170, 70, 8, 0, oro_osc)
estrella(150, 62, 8, math.pi / 8, oro)
d.ellipse((ecx - 46, ecy - 46, ecx + 46, ecy + 46), fill=(20, 15, 10), outline=oro, width=4)
estrella(30, 12, 4, 0, oro)

# Adornos en las esquinas del marco.
for (x, y) in [(m2 + 30, m2 + 30), (W - m2 - 30, m2 + 30), (m2 + 30, H - m2 - 30), (W - m2 - 30, H - m2 - 30)]:
    d.regular_polygon((x, y, 12), 4, rotation=45, fill=oro)

img = img.filter(ImageFilter.SMOOTH_MORE)
out = Path(__file__).resolve().parent.parent / "src-tauri" / "assets" / "reverso-generico.png"
img.save(out, optimize=True)
print(out, out.stat().st_size)
