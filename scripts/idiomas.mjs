#!/usr/bin/env node
// Comprueba el idioma de la interfaz (versión reducida de la de judlex).
//
//   node scripts/idiomas.mjs          # falla si falta alguna traducción
//
// El diccionario (`src/i18n/en/`, un fichero por área) se indexa por la CADENA
// FUENTE EN ESPAÑOL y `t()` devuelve el español cuando no encuentra la entrada.
// Es un respaldo deliberado, pero hace que una traducción que falta NO se note
// programando: todo compila y pinta, y el hueco solo se ve con la app en
// inglés. Este script es la forma de enterarse.
//
// Cuenta como «pedido» todo literal dentro de `t("…")`, `t(\`…\`)` sin
// interpolación, y `paraTraducir("…")`.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const RAIZ = new URL("..", import.meta.url).pathname;
const DICCIONARIO = join(RAIZ, "src/i18n/en");

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

// ── Las claves del diccionario, y los choques entre áreas ──────────────────
const claves = new Map();
const choques = [];
for (const f of readdirSync(DICCIONARIO)) {
	if (f === "index.ts" || !f.endsWith(".ts")) continue;
	const texto = readFileSync(join(DICCIONARIO, f), "utf8");
	for (const m of texto.matchAll(
		/^\s*(?:"((?:[^"\\]|\\.)*)"|([A-Za-zÁÉÍÓÚÑáéíóúñ][\wÁÉÍÓÚÑáéíóúñ]*))\s*:/gm,
	)) {
		const k = desescapar(m[1] ?? m[2]);
		if (claves.has(k)) choques.push(`"${k}" · ${claves.get(k)} y ${f}`);
		else claves.set(k, f);
	}
}

// ── Lo que la app pide traducir ─────────────────────────────────────────────
const pedidas = new Map();
for (const f of fuentes(join(RAIZ, "src"))) {
	if (f.startsWith(DICCIONARIO)) continue;
	const texto = readFileSync(f, "utf8");
	const patrones = [/\bt\(\s*"((?:[^"\\]|\\.)*)"/g, /\bparaTraducir\(\s*"((?:[^"\\]|\\.)*)"/g];
	for (const re of patrones) {
		for (const m of texto.matchAll(re)) {
			const k = desescapar(m[1]);
			if (!pedidas.has(k)) pedidas.set(k, f.slice(RAIZ.length));
		}
	}
}

const faltan = [...pedidas].filter(([k]) => !claves.has(k));
let mal = false;
if (choques.length) {
	mal = true;
	console.error(`✖ ${choques.length} cadena(s) en dos áreas del diccionario:`);
	for (const c of choques) console.error(`  ${c}`);
}
if (faltan.length) {
	mal = true;
	console.error(`✖ ${faltan.length} cadena(s) sin traducir al inglés:`);
	for (const [k, f] of faltan) console.error(`  "${k}"  ← ${f}`);
}
if (mal) process.exit(1);
console.log(`✓ idiomas: ${pedidas.size} cadenas, todas traducidas.`);
