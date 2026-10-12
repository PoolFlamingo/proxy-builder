"""Genera el icono de la app (1024×1024) a partir de formas simples.

Diseño propio: dos cartas superpuestas en dorado sobre fondo oscuro, sin
ninguna marca de terceros. Se ejecuta a mano cuando cambie el diseño:

    python3 scripts/icono.py && pnpm tauri icon src-tauri/icons/icono-1024.png
"""
from pathlib import Path

from PIL import Image, ImageDraw

S = 1024
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
# Fondo redondeado.
d.rounded_rectangle((32, 32, S - 32, S - 32), radius=200, fill=(22, 20, 18, 255))
oro = (214, 170, 72, 255)
oro_osc = (150, 112, 40, 255)


def carta(cx, cy, ang, relleno, borde):
    w, h = 400, 560
    c = Image.new("RGBA", (w + 40, h + 40), (0, 0, 0, 0))
    cd = ImageDraw.Draw(c)
    cd.rounded_rectangle((20, 20, w + 20, h + 20), radius=36, fill=relleno, outline=borde, width=14)
    cd.rounded_rectangle((70, 80, w - 30, h - 220), radius=18, outline=borde, width=10)
    for i in range(3):
        y = h - 170 + i * 50
        cd.rounded_rectangle((70, y, w - 30 - i * 60, y + 22), radius=11, fill=borde)
    c = c.rotate(ang, resample=Image.BICUBIC, expand=True)
    img.alpha_composite(c, (cx - c.width // 2, cy - c.height // 2))


carta(450, 540, 12, (40, 34, 26, 255), oro_osc)
carta(580, 500, -8, (30, 26, 22, 255), oro)

out = Path(__file__).resolve().parent.parent / "src-tauri" / "icons" / "icono-1024.png"
out.parent.mkdir(parents=True, exist_ok=True)
img.save(out)
print(out)
