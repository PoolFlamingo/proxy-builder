#!/usr/bin/env node
/**
 * Falla si algún `.ts`/`.tsx` de `src/` escucha algo que solo existe con el
 * ratón (entrar, salir, pasar por encima, clic derecho) sin decir cómo se hace
 * con el dedo.
 *
 * Lo vigilado: los props `onMouseEnter`, `onPointerEnter`, `onMouseLeave`,
 * `onPointerLeave`, `onMouseOver`, `onPointerOver` y `onContextMenu`, y los
 * `addEventListener` de `mouseenter`, `pointerenter`, `mouseover`,
 * `pointerover` y `contextmenu`.
 *
 * 🛑 Por qué: en una pantalla táctil no hay «ratón encima» ni clic derecho, y
 * lo que solo salía así se quedaba inalcanzable (plan del móvil, 02/11/2026).
 * Cada uso lleva, en su línea o en las tres anteriores, un comentario
 * `// táctil: …` (o `{/* táctil: … *​/}`) que dice cómo se llega con el dedo
 * (pulsación larga, un toque, un botón «⋯»…). Mientras no esté hecho, el
 * comentario dice «pendiente de M3.x».
 *
 * No se miran `components/ui/` ni `components/tooltips-nativos.tsx`, que son de
 * solo lectura (heredados de judlex). Este último escucha `pointerover` en todo
 * el documento para los `title`: con el dedo no hay tooltip, y el nombre de un
 * botón de icono sale al mantenerlo pulsado (`components/etiquetas-tactiles.tsx`).
 *
 * Uso: `node scripts/sin-hover-solo.mjs` (va en `pnpm check`).
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const RAIZ = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const PROP =
	/\b(onMouseEnter|onPointerEnter|onMouseLeave|onPointerLeave|onMouseOver|onPointerOver|onContextMenu)\b\s*[=:]/;
const ESCUCHA =
	/addEventListener\(\s*["'`](mouseenter|pointerenter|mouseover|pointerover|contextmenu)["'`]/;
const MARCA = /táctil:/;

const SOLO_LECTURA = new Set([join("components", "tooltips-nativos.tsx")]);

function* ficheros(dir) {
	for (const nombre of readdirSync(dir)) {
		const ruta = join(dir, nombre);
		if (statSync(ruta).isDirectory()) {
			if (
				nombre === "__tests__" ||
				nombre === "bindings" ||
				ruta.endsWith(join("components", "ui"))
			)
				continue;
			yield* ficheros(ruta);
		} else if (
			/\.tsx?$/.test(ruta) &&
			!/\.test\.tsx?$/.test(ruta) &&
			![...SOLO_LECTURA].some((x) => ruta.endsWith(x))
		) {
			yield ruta;
		}
	}
}

/** Las líneas (1-based) que escuchan algo solo de ratón y no dicen cómo se hace con el dedo. */
export function sinMarca(texto) {
	const lineas = texto.split("\n");
	const mal = [];
	lineas.forEach((l, i) => {
		const t = l.trim();
		if (t.startsWith("//") || t.startsWith("*") || t.startsWith("/*")) return;
		if (!PROP.test(l) && !ESCUCHA.test(l)) return;
		const desde = Math.max(0, i - 3);
		if (lineas.slice(desde, i + 1).some((x) => MARCA.test(x))) return;
		mal.push(i + 1);
	});
	return mal;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	let fallos = 0;
	for (const f of ficheros(join(RAIZ, "src"))) {
		for (const n of sinMarca(readFileSync(f, "utf8"))) {
			console.error(
				`${relative(RAIZ, f)}:${n}  solo de ratón: falta «// táctil: …» (cómo se hace con el dedo)`,
			);
			fallos++;
		}
	}
	if (fallos) {
		console.error(
			`\n${fallos} sitio(s) que solo funcionan con el ratón. Ver scripts/sin-hover-solo.mjs.`,
		);
		process.exit(1);
	}
}
