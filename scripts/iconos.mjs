#!/usr/bin/env node
/**
 * **Los iconos de la app, dentro de la app.** Busca en `src/` cada id de
 * Iconify escrito tal cual (`"lucide:crown"`, `"game-icons:crossed-swords"`),
 * saca sus datos de `@iconify/json` (con los alias resueltos) y escribe
 * `src/lib/iconos/incluidos.ts`: el `<Icono>` los pinta como un `<svg>` sin
 * esperar a nada. Los iconos que elige la persona (el selector) no van aquí:
 * los baja Rust (`iconos.rs`) y se guardan en `app_data_dir/iconos/`.
 *
 *   node scripts/iconos.mjs          escribe el fichero
 *   node scripts/iconos.mjs --check  falla si el fichero no está al día
 *
 * Los ids que se montan con texto (`` `lucide:${x}` ``) no se ven: se ponen
 * escritos enteros en algún sitio (una tabla) para que entren.
 */
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { extname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { getIconData, iconToSVG } from "@iconify/utils";

const RAIZ = join(fileURLToPath(import.meta.url), "..", "..");
const SALIDA = join(RAIZ, "src", "lib", "iconos", "incluidos.ts");
const JSON_DIR = join(RAIZ, "node_modules", "@iconify", "json", "json");
const SALTAR = new Set(["node_modules", "bindings", "assets", "__tests__"]);
const EXTENSIONES = new Set([".ts", ".tsx"]);
// Cualquier `prefijo:nombre` entre comillas; los que no son iconos (`16:9`,
// `pbimg:…`) se caen solos porque su prefijo no es una colección o el nombre
// no está en ella.
const ID = /["'`]([a-z][a-z0-9-]*:[a-z0-9][a-z0-9-]*)["'`]/g;

function recorrer(dir, ids) {
	for (const e of readdirSync(dir)) {
		if (SALTAR.has(e)) continue;
		const ruta = join(dir, e);
		if (statSync(ruta).isDirectory()) recorrer(ruta, ids);
		else if (EXTENSIONES.has(extname(e)) && !/\.test\.tsx?$/.test(e) && ruta !== SALIDA)
			for (const m of readFileSync(ruta, "utf8").matchAll(ID)) ids.add(m[1]);
	}
}

const colecciones = new Map();
function coleccion(prefijo) {
	if (!colecciones.has(prefijo)) {
		const ruta = join(JSON_DIR, `${prefijo}.json`);
		colecciones.set(prefijo, existsSync(ruta) ? JSON.parse(readFileSync(ruta, "utf8")) : null);
	}
	return colecciones.get(prefijo);
}

const ids = new Set();
recorrer(join(RAIZ, "src"), ids);
const datos = {};
const faltan = [];
for (const id of [...ids].sort()) {
	const [prefijo, nombre] = id.split(":");
	const c = coleccion(prefijo);
	if (!c) continue;
	const icono = getIconData(c, nombre);
	if (!icono) {
		faltan.push(id);
		continue;
	}
	const svg = iconToSVG(icono);
	datos[id] = { body: svg.body, vb: svg.attributes.viewBox };
}

const texto = `// GENERADO por scripts/iconos.mjs (\`pnpm iconos\`): no se edita a mano.
// Los iconos que la app escribe tal cual, listos para pintar sin esperar.

export const INCLUIDOS: Record<string, { body: string; vb: string }> = ${JSON.stringify(datos)};
`;

if (faltan.length) {
	console.error(`Iconos que no existen: ${faltan.join(", ")}`);
	process.exit(1);
}
const rel = relative(RAIZ, SALIDA);
if (process.argv.includes("--check")) {
	const actual = existsSync(SALIDA) ? readFileSync(SALIDA, "utf8") : "";
	if (actual !== texto) {
		console.error(`${rel} no está al día: \`pnpm iconos\`.`);
		process.exit(1);
	}
} else {
	writeFileSync(SALIDA, texto);
	console.log(`${rel}: ${Object.keys(datos).length} iconos.`);
}
