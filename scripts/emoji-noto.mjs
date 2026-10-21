#!/usr/bin/env node
// Copia a `src/assets/emoji/` los SVG de Noto Color Emoji que usa la interfaz.
//
//   node scripts/emoji-noto.mjs                       # las banderas de src/lib/idiomas.ts
//   node scripts/emoji-noto.mjs 😀 ✨                  # además, estos emoji
//   node scripts/emoji-noto.mjs --desde <clon> …      # de un clon local, sin red
//
// Siempre del mismo commit de `googlefonts/noto-emoji`: las banderas de
// `third_party/region-flags/waved-svg/` (dominio público) y el resto de
// `2D/svg/` (Apache 2.0). Escribe `fuentes.json` con el commit y el SHA-256 de
// cada fichero, para saber de dónde salió cada uno. No borra los que ya hay.
//
// Un SVG de más de 64 KB (la bandera de España, por su escudo, pesa 415 KB)
// se guarda en PNG de 128 px de ancho, pasado por Inkscape: en la interfaz va
// a 1em y el detalle no se ve. Hace falta `inkscape` en el PATH.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const COMMIT = "e20cbc2bbec1926686be9f9bee7d1d2cfa1fea0e";
const RAIZ = fileURLToPath(new URL("..", import.meta.url));
const DESTINO = join(RAIZ, "src/assets/emoji");
const MAX_SVG = 64 * 1024;

/** El nombre de fichero de Noto: los puntos de código sin el selector de variante. */
export function nombreNoto(emoji) {
	return `emoji_u${[...emoji]
		.map((c) => c.codePointAt(0))
		.filter((p) => p !== 0xfe0f)
		.map((p) => p.toString(16).padStart(4, "0"))
		.join("_")}.svg`;
}

/** Dos letras de país → su bandera (los dos indicadores regionales). */
export function bandera(pais) {
	return [...pais.toUpperCase()]
		.map((c) => String.fromCodePoint(0x1f1e6 + c.charCodeAt(0) - 65))
		.join("");
}

const esBandera = (emoji) =>
	[...emoji].every((c) => (c.codePointAt(0) ?? 0) >= 0x1f1e6 && (c.codePointAt(0) ?? 0) <= 0x1f1ff);

async function paises() {
	const js = ts.transpileModule(readFileSync(join(RAIZ, "src/lib/idiomas.ts"), "utf8"), {
		compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
	}).outputText;
	const { IDIOMAS } = await import(
		`data:text/javascript;base64,${Buffer.from(js).toString("base64")}`
	);
	return IDIOMAS.map((i) => i.pais);
}

async function leer(ruta, desde) {
	if (desde) return readFileSync(join(desde, ruta));
	const r = await fetch(
		`https://raw.githubusercontent.com/googlefonts/noto-emoji/${COMMIT}/${ruta}`,
	);
	if (!r.ok) throw new Error(`${ruta}: HTTP ${r.status}`);
	return Buffer.from(await r.arrayBuffer());
}

async function main() {
	const args = process.argv.slice(2);
	const i = args.indexOf("--desde");
	const desde = i >= 0 ? args.splice(i, 2)[1] : null;
	const emoji = [...(await paises()).map(bandera), ...args];
	mkdirSync(DESTINO, { recursive: true });
	const indice = join(DESTINO, "fuentes.json");
	const fuentes = existsSync(indice)
		? JSON.parse(readFileSync(indice, "utf8"))
		: { commit: COMMIT, ficheros: {} };
	if (fuentes.commit !== COMMIT)
		throw new Error(`fuentes.json es de otro commit (${fuentes.commit})`);
	for (const e of emoji) {
		const nombre = nombreNoto(e);
		const ruta = esBandera(e) ? `third_party/region-flags/waved-svg/${nombre}` : `2D/svg/${nombre}`;
		const datos = await leer(ruta, desde);
		const sha256 = createHash("sha256").update(datos).digest("hex");
		let fichero = nombre;
		if (datos.length > MAX_SVG) {
			fichero = nombre.replace(/\.svg$/, ".png");
			const svg = join(DESTINO, nombre);
			writeFileSync(svg, datos);
			execFileSync("inkscape", [svg, "-w", "128", "-o", join(DESTINO, fichero)], {
				stdio: "ignore",
			});
			rmSync(svg);
		} else writeFileSync(join(DESTINO, fichero), datos);
		delete fuentes.ficheros[fichero === nombre ? nombre.replace(/\.svg$/, ".png") : nombre];
		fuentes.ficheros[fichero] = {
			ruta,
			sha256,
			...(fichero !== nombre ? { png: "128 px, Inkscape" } : {}),
		};
		console.log(`✓ ${e}  ${fichero}  ${readFileSync(join(DESTINO, fichero)).length} B`);
	}
	fuentes.ficheros = Object.fromEntries(
		Object.entries(fuentes.ficheros).sort(([a], [b]) => a.localeCompare(b)),
	);
	writeFileSync(indice, `${JSON.stringify(fuentes, null, "\t")}\n`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) await main();
