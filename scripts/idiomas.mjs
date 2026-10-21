#!/usr/bin/env node
// Comprueba los idiomas de la interfaz (versión ampliada de la de judlex).
//
//   node scripts/idiomas.mjs            # falla si a algún idioma le falta algo
//   node scripts/idiomas.mjs --detalle  # además, lista cada aviso
//
// Cada diccionario (`src/i18n/<código>/`, un JSON por área, que `cargar.ts`
// funde) se indexa por la CADENA FUENTE EN ESPAÑOL y `t()` cae al inglés (y
// luego al español) cuando no encuentra la entrada. Es un respaldo deliberado,
// pero hace que una traducción que falta NO se note programando: todo compila
// y pinta, y el hueco solo se ve con la app en ese idioma. Este script es la
// forma de enterarse.
//
// Cuenta como «pedido» todo literal dentro de `t("…")` y `paraTraducir("…")`.
// Por idioma comprueba (fallo): que no falte ninguna cadena, que ninguna esté
// en dos áreas ni dos veces en la misma, que cada traducción (y cada forma de
// un plural) traiga los mismos `{x}` que la española, que cada plural tenga
// todas las categorías de `Intl.PluralRules` de su idioma y que solo lleven
// plural las cadenas con `{n}`. Avisa (sin fallar) de las que sobran y de las
// idénticas al inglés. Las que deben ser iguales al inglés en un idioma («Deck»
// en alemán) van en `src/i18n/<código>/identicas.json` (una lista de cadenas
// fuente).

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const RAIZ = fileURLToPath(new URL("..", import.meta.url));
const I18N = join(RAIZ, "src/i18n");

/** Lo que puede ser igual en inglés y en otro idioma sin ser un despiste. */
const IDENTICAS_PERMITIDAS = new Set([
	"OK",
	"PDF",
	"PNG",
	"JPEG",
	"WebP",
	"TIFF",
	"BMP",
	"Scryfall",
	"Moxfield",
	"MTGO",
	"Arena",
	"MTGJSON",
	"Cockatrice",
	"Card Conjurer",
	"tweakcn",
	"Commander",
	"Proxy Builder",
]);

const CATEGORIAS = new Set(["zero", "one", "two", "few", "many", "other"]);

function fuentes(dir, salida = []) {
	for (const nombre of readdirSync(dir)) {
		if (nombre === "node_modules" || nombre.startsWith(".") || nombre === "bindings") continue;
		const p = join(dir, nombre);
		if (statSync(p).isDirectory()) fuentes(p, salida);
		else if (/\.tsx?$/.test(nombre) && !/\.test\.tsx?$/.test(nombre)) salida.push(p);
	}
	return salida;
}

const desescapar = (s) => s.replace(/\\"/g, '"').replace(/\\n/g, "\n");

/** Un `.ts` sin más imports que de tipos, evaluado a sus exportaciones. */
async function importarTs(fichero) {
	const js = ts.transpileModule(readFileSync(fichero, "utf8"), {
		compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
	}).outputText;
	return import(`data:text/javascript;base64,${Buffer.from(js).toString("base64")}`);
}

/** Lo que la app pide traducir: cadena → fichero donde se pidió primero. */
export function leerPedidas(raiz = RAIZ) {
	const pedidas = new Map();
	for (const f of fuentes(join(raiz, "src"))) {
		if (f.startsWith(I18N)) continue;
		const texto = readFileSync(f, "utf8");
		const patrones = [/\bt\(\s*"((?:[^"\\]|\\.)*)"/g, /\bparaTraducir\(\s*"((?:[^"\\]|\\.)*)"/g];
		for (const re of patrones) {
			for (const m of texto.matchAll(re)) {
				const k = desescapar(m[1]);
				if (!pedidas.has(k)) pedidas.set(k, f.slice(raiz.length));
			}
		}
	}
	return pedidas;
}

/** Los códigos de `src/lib/idiomas.ts` y su locale. */
export async function leerTabla() {
	const { IDIOMAS } = await importarTs(join(RAIZ, "src/lib/idiomas.ts"));
	return IDIOMAS.map((i) => ({ codigo: i.codigo, locale: i.locale }));
}

/**
 * Los diccionarios en disco: código → { areas: { area: objeto }, identicas,
 * repetidas }. Cada área es un `<área>.json` de la carpeta del idioma.
 * `repetidas` son las cadenas que salen dos veces en el mismo fichero:
 * `JSON.parse` se las come en silencio (el TS de antes no las dejaba pasar).
 */
export function leerDiccionarios(dir = I18N) {
	const salida = {};
	for (const codigo of readdirSync(dir)) {
		const p = join(dir, codigo);
		if (!statSync(p).isDirectory() || codigo.startsWith("_")) continue;
		const areas = {};
		const repetidas = [];
		for (const f of readdirSync(p)) {
			if (f === "identicas.json" || !f.endsWith(".json")) continue;
			const area = f.replace(/\.json$/, "");
			const texto = readFileSync(join(p, f), "utf8");
			areas[area] = JSON.parse(texto);
			const vistas = new Set();
			for (const m of texto.matchAll(/^\t"((?:[^"\\]|\\.)*)"\s*:/gm)) {
				if (vistas.has(m[1])) repetidas.push(`repetida en ${area}: "${desescapar(m[1])}"`);
				vistas.add(m[1]);
			}
		}
		let identicas = [];
		try {
			identicas = JSON.parse(readFileSync(join(p, "identicas.json"), "utf8"));
		} catch {
			/* sin excepciones */
		}
		salida[codigo] = { areas, identicas, repetidas };
	}
	return salida;
}

const marcadores = (s) => [...s.matchAll(/\{([^{}\s]*)\}/g)].map((m) => m[1]).sort();
const mismos = (a, b) => a.length === b.length && a.every((x, i) => x === b[i]);

/**
 * Comprueba un idioma. Devuelve `{ errores, avisos, total }` (listas de
 * textos). `en` es el diccionario inglés fundido, para el aviso de idénticas.
 */
export function comprobar({
	codigo,
	locale,
	areas,
	repetidas = [],
	pedidas,
	areasEn,
	en,
	identicas = [],
}) {
	const permitidas = new Set(identicas);
	const errores = [...repetidas];
	const avisos = [];
	const fundido = new Map();
	for (const [area, dic] of Object.entries(areas)) {
		for (const [k, v] of Object.entries(dic)) {
			if (fundido.has(k)) errores.push(`en dos áreas (${fundido.get(k).area} y ${area}): "${k}"`);
			else fundido.set(k, { area, v });
		}
	}
	for (const area of areasEn) {
		if (!(area in areas)) errores.push(`falta el área ${area}.json`);
	}
	const categorias = new Intl.PluralRules(locale).resolvedOptions().pluralCategories;
	for (const [k, origen] of pedidas) {
		if (!fundido.has(k)) errores.push(`falta "${k}"  ← ${origen}`);
	}
	for (const [k, { v }] of fundido) {
		if (!pedidas.has(k)) {
			avisos.push(`sobra "${k}"`);
			continue;
		}
		const esperados = marcadores(k);
		if (typeof v === "string") {
			if (!mismos(marcadores(v), esperados)) errores.push(`marcadores distintos en "${k}": "${v}"`);
			const ingles = en?.[k];
			if (
				codigo !== "en" &&
				typeof ingles === "string" &&
				v === ingles &&
				ingles !== k &&
				/\p{L}/u.test(v.replace(/\{[^}]*\}/g, "")) &&
				!IDENTICAS_PERMITIDAS.has(v) &&
				!permitidas.has(k)
			)
				avisos.push(`idéntica al inglés: "${k}" → "${v}"`);
			continue;
		}
		if (!v || typeof v !== "object" || typeof v.other !== "string") {
			errores.push(`valor no válido en "${k}"`);
			continue;
		}
		if (!esperados.includes("n")) errores.push(`plural en una cadena sin {n}: "${k}"`);
		for (const [cat, forma] of Object.entries(v)) {
			if (!CATEGORIAS.has(cat) || !categorias.includes(cat))
				errores.push(`categoría de plural «${cat}» que ${codigo} no tiene: "${k}"`);
			else if (!mismos(marcadores(forma), esperados))
				errores.push(`marcadores distintos en "${k}" (${cat}): "${forma}"`);
		}
		for (const cat of categorias) {
			if (!(cat in v)) errores.push(`al plural le falta «${cat}»: "${k}"`);
		}
	}
	return { errores, avisos, total: pedidas.size };
}

/**
 * El español es la fuente: su diccionario repite cada cadena como su propia
 * traducción. Así `src/i18n/es/` tiene los mismos ficheros y claves que el
 * inglés, y una cadena nueva de `t()` no se escapa sin entrada en español.
 */
export function comprobarFuente({ areas, repetidas = [], pedidas, areasEn }) {
	const errores = [...repetidas];
	const avisos = [];
	const vistas = new Set();
	for (const [area, dic] of Object.entries(areas)) {
		for (const [k, v] of Object.entries(dic)) {
			if (vistas.has(k)) errores.push(`en dos áreas (${area}): "${k}"`);
			vistas.add(k);
			if (v !== k) errores.push(`en es, el valor no es la fuente: "${k}" → ${JSON.stringify(v)}`);
		}
	}
	for (const area of areasEn) {
		if (!(area in areas)) errores.push(`falta el área ${area}.json`);
	}
	for (const [k, origen] of pedidas) {
		if (!vistas.has(k)) errores.push(`falta "${k}"  ← ${origen}`);
	}
	for (const k of vistas) {
		if (!pedidas.has(k)) avisos.push(`sobra "${k}"`);
	}
	return { errores, avisos, total: pedidas.size };
}

async function main() {
	const detalle = process.argv.includes("--detalle");
	const pedidas = leerPedidas();
	const tabla = await leerTabla();
	const dics = leerDiccionarios();
	const areasEn = Object.keys(dics.en?.areas ?? {});
	const en = Object.assign({}, ...Object.values(dics.en?.areas ?? {}));
	let mal = false;
	for (const { codigo, locale } of tabla) {
		const d = dics[codigo];
		if (!d) {
			mal = true;
			console.error(`✖ ${codigo}: no hay diccionario (src/i18n/${codigo}/)`);
			continue;
		}
		const { errores, avisos, total } =
			codigo === "es"
				? comprobarFuente({ ...d, pedidas, areasEn })
				: comprobar({
						codigo,
						locale,
						...d,
						pedidas,
						areasEn,
						en,
					});
		const nota = avisos.length ? ` (${avisos.length} aviso(s))` : "";
		if (errores.length) {
			mal = true;
			console.error(`✖ ${codigo}: ${errores.length} fallo(s)${nota}`);
			for (const e of errores) console.error(`  ${e}`);
		} else console.log(`✓ ${codigo}: ${total} cadenas${nota}`);
		if (detalle) for (const a of avisos) console.log(`  · ${a}`);
	}
	for (const codigo of Object.keys(dics)) {
		if (!tabla.some((i) => i.codigo === codigo)) {
			mal = true;
			console.error(`✖ ${codigo}: el directorio no es de ningún idioma de src/lib/idiomas.ts`);
		}
	}
	if (mal) process.exit(1);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) await main();
