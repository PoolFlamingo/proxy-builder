#!/usr/bin/env python3
"""Añade cadenas a los 15 diccionarios de una área, sin pisarse si varios
procesos lo hacen a la vez (bloqueo con flock).

Uso:
    python3 scripts/i18n-anadir.py <area> <<'JSON'
    { "Texto en español": { "en": "…", "fr": "…", "de": "…", "it": "…", "pt": "…",
        "ro": "…", "pl": "…", "nl": "…", "ru": "…", "uk": "…", "ja": "…",
        "ko": "…", "zhs": "…", "zht": "…" } }
    JSON

`es` (la fuente) repite la cadena sola. Si además lleva `"es": { "one": …,
"many": …, "other": … }` (su `other` es la propia cadena), ese plural va a
`src/i18n/plurales-es.json`, lo único del español que carga la app. Un plural (`{n}` que cuenta) lleva en
cada idioma un objeto `{ "one": …, "few": …, "many": …, "other": … }` con las
categorías de `Intl.PluralRules` de ese idioma. Si la clave ya existe se deja
como está. Luego, `pnpm idiomas` comprueba que no falta nada.
"""
import collections
import fcntl
import json
import pathlib
import sys

LANGS = ["es", "en", "fr", "de", "it", "pt", "ro", "pl", "nl", "ru", "uk", "ja", "ko", "zhs", "zht"]
RAIZ = pathlib.Path(__file__).resolve().parent.parent / "src" / "i18n"


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    area = sys.argv[1]
    nuevas = json.load(sys.stdin, object_pairs_hook=collections.OrderedDict)
    for clave, tr in nuevas.items():
        faltan = [l for l in LANGS if l != "es" and l not in tr]
        if faltan:
            sys.exit(f"«{clave}»: faltan {faltan}")
    with open(RAIZ / ".bloqueo", "w") as cerrojo:
        fcntl.flock(cerrojo, fcntl.LOCK_EX)
        for lang in LANGS:
            ruta = RAIZ / lang / f"{area}.json"
            if ruta.exists():
                d = json.loads(ruta.read_text(encoding="utf-8"), object_pairs_hook=collections.OrderedDict)
            else:
                d = collections.OrderedDict()
            for clave, tr in nuevas.items():
                if clave not in d:
                    d[clave] = clave if lang == "es" else tr[lang]
            ruta.write_text(json.dumps(d, ensure_ascii=False, indent="\t") + "\n", encoding="utf-8")
        plurales = {c: tr["es"] for c, tr in nuevas.items() if isinstance(tr.get("es"), dict)}
        if plurales:
            ruta = RAIZ / "plurales-es.json"
            d = json.loads(ruta.read_text(encoding="utf-8"), object_pairs_hook=collections.OrderedDict)
            for clave, forma in plurales.items():
                d.setdefault(clave, forma)
            d = collections.OrderedDict(sorted(d.items()))
            ruta.write_text(json.dumps(d, ensure_ascii=False, indent="\t") + "\n", encoding="utf-8")
        fcntl.flock(cerrojo, fcntl.LOCK_UN)


main()
