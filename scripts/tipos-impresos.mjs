#!/usr/bin/env node
// Baja de Scryfall cómo se imprimen en cada idioma las líneas de tipo, para que
// el motor las traduzca sin red cuando una carta no tiene impresión en el idioma
// de diseño.
//
//   node scripts/tipos-impresos.mjs            # todos los idiomas con cartas
//   node scripts/tipos-impresos.mjs fr de      # solo estos
//
// Recorre las cartas de cada idioma (`lang:xx`, con fichas, una impresión por carta) y, de
// cada cara, empareja la parte de tipos en inglés («Legendary Artifact
// Creature») con la impresa («Criatura artefacto legendaria»), y cada subtipo
// suelto en inglés con el impreso. Entre varias formas se queda con la más
// repetida (la más reciente si empatan). Escribe
// `src-tauri/crates/proxy-core/assets/tipos-impresos.json` y avisa de los tipos
// y subtipos de los catálogos de Scryfall que un idioma no tiene.
//
// Los idiomas sin cartas impresas (ro, pl, nl, uk) no están: ahí manda la tabla
// escrita a mano de `tipos_idiomas.rs`.

import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const RAIZ = fileURLToPath(new URL("..", import.meta.url));
const SALIDA = join(RAIZ, "src-tauri/crates/proxy-core/assets/tipos-impresos.json");
const IDIOMAS = ["es", "fr", "de", "it", "pt", "ru", "ja", "ko", "zhs", "zht"];
const CATALOGOS = [
	"creature-types",
	"land-types",
	"artifact-types",
	"enchantment-types",
	"spell-types",
	"planeswalker-types",
	"battle-types",
];
const SEPARADOR = /\s*[—―～]\s*/u;

const cabeceras = {
	"User-Agent": "proxy-builder-tipos/1.0 (https://github.com/PoolFlamingo/proxy-builder)",
	Accept: "application/json",
};

/** Un hueco de 110 ms entre peticiones, aunque vayan varios idiomas a la vez (Scryfall pide 10/s como mucho). */
let ultima = Promise.resolve();
function turno() {
	const mio = ultima.then(() => new Promise((r) => setTimeout(r, 110)));
	ultima = mio;
	return mio;
}

async function pedir(url) {
	for (let intento = 0; intento < 12; intento++) {
		await turno();
		try {
			const r = await fetch(url, { headers: cabeceras, signal: AbortSignal.timeout(60_000) });
			if (r.ok) return r.json();
			if (r.status === 404) return null;
			console.error(`HTTP ${r.status} en ${url}`);
		} catch (e) {
			console.error(`${e.message} en ${url}`);
		}
		await new Promise((r) => setTimeout(r, Math.min(30_000, 2000 * (intento + 1))));
	}
	throw new Error(`Scryfall no responde: ${url}`);
}

/** Cuenta una forma impresa; la de más veces gana y, a igualdad, la que salió antes. */
function contar(mapa, clave, valor) {
	const m = mapa.get(clave) ?? new Map();
	mapa.set(clave, m);
	m.set(valor, (m.get(valor) ?? 0) + 1);
}

function ganadora(m) {
	let mejor = null;
	let veces = 0;
	for (const [v, n] of m) {
		if (n > veces) {
			mejor = v;
			veces = n;
		}
	}
	return mejor;
}

const CONJUNCIONES = new Set(["et", "y", "e", "und", "и", "and"]);

/**
 * Los subtipos que ninguna carta trae suelto («Druid») se sacan de las líneas
 * con varios («Elf Druid» → «Elfo Druida»): si de una línea solo falta un
 * subtipo y en lo impreso sobra una sola palabra, esa es. Se repite hasta que
 * no sale nada nuevo.
 */
function aprenderDeVarios(subtipos, multiples) {
	const sabido = (en) => (subtipos.has(en) ? ganadora(subtipos.get(en)) : null);
	for (let cambio = true; cambio; ) {
		cambio = false;
		for (const { en, imp } of multiples) {
			const palabras = imp.split(/[\s,、・／/]+/u).filter((p) => p && !CONJUNCIONES.has(p));
			if (palabras.length !== en.length) continue;
			const sinSaber = en.filter((w) => !sabido(w));
			if (sinSaber.length !== 1) continue;
			const resto = palabras.filter((p) => !en.some((w) => sabido(w) === p));
			if (resto.length !== 1) continue;
			contar(subtipos, sinSaber[0], resto[0]);
			cambio = true;
		}
	}
}

/** Las caras con su tipo en inglés y su tipo impreso. */
function caras(c) {
	const lista = c.card_faces?.some((f) => f.printed_type_line) ? c.card_faces : [c];
	return lista
		.map((f) => ({ en: f.type_line ?? c.type_line, impreso: f.printed_type_line }))
		.filter((f) => f.en && f.impreso && !f.en.includes(" // "));
}

async function idiomaEntero(lang) {
	const tipos = new Map();
	const subtipos = new Map();
	const multiples = [];
	const q = new URLSearchParams({
		q: `lang:${lang}`,
		unique: "cards",
		order: "released",
		dir: "desc",
		include_extras: "true",
		include_multilingual: "true",
	});
	let url = `https://api.scryfall.com/cards/search?${q}`;
	let pagina = 0;
	while (url) {
		const r = await pedir(url);
		if (!r) break;
		for (const c of r.data) {
			if (c.lang !== lang) continue;
			for (const f of caras(c)) {
				const [tipoEn, ...subEn] = f.en.split(" — ");
				const [tipoImp, ...subImp] = f.impreso.split(SEPARADOR);
				if (subEn.length !== subImp.length || !tipoImp.trim()) continue;
				contar(tipos, tipoEn.trim().replace(/\s+/g, " "), tipoImp.trim());
				if (subEn.length === 1 && /\s/.test(subEn[0].trim())) {
					multiples.push({ en: subEn[0].trim().split(/\s+/), imp: subImp[0].trim() });
				}
				const unoEn = subEn[0]?.trim();
				const unoImp = subImp[0]?.trim();
				if (unoEn && unoImp && !/\s/.test(unoEn) && !/[\s,、・／/]/.test(unoImp)) {
					contar(subtipos, unoEn, unoImp);
				}
			}
		}
		url = r.has_more ? r.next_page : null;
		if (++pagina % 20 === 0) console.error(`${lang}: ${pagina} páginas`);
	}
	aprenderDeVarios(subtipos, multiples);
	console.error(`${lang}: ${tipos.size} tipos, ${subtipos.size} subtipos`);
	const orden = (m) =>
		Object.fromEntries(
			[...m].sort(([a], [b]) => a.localeCompare(b)).map(([k, v]) => [k, ganadora(v)]),
		);
	return { tipos: orden(tipos), subtipos: orden(subtipos) };
}

const pedidos = process.argv.slice(2);
const lista = pedidos.length ? pedidos : IDIOMAS;
let previo = { idiomas: {} };
try {
	previo = JSON.parse(readFileSync(SALIDA, "utf8"));
} catch {}

const catalogos = new Map();
for (const c of CATALOGOS)
	catalogos.set(c, (await pedir(`https://api.scryfall.com/catalog/${c}`))?.data ?? []);
const supertipos = (await pedir("https://api.scryfall.com/catalog/supertypes")).data;
const tiposDeCarta = (await pedir("https://api.scryfall.com/catalog/card-types")).data;

const idiomas = { ...previo.idiomas };

/** Se guarda al acabar cada idioma: si se corta, lo ya bajado no se pierde. */
function guardar() {
	const salida = {
		fuente:
			"Scryfall: tipo en inglés y tipo impreso de cada cara, por idioma (scripts/tipos-impresos.mjs)",
		idiomas: Object.fromEntries(Object.entries(idiomas).sort(([a], [b]) => a.localeCompare(b))),
	};
	writeFileSync(SALIDA, `${JSON.stringify(salida, null, "\t")}\n`);
}

// De tres en tres: cada idioma son cientos de páginas.
for (let i = 0; i < lista.length; i += 3) {
	await Promise.all(
		lista.slice(i, i + 3).map(async (lang) => {
			idiomas[lang] = await idiomaEntero(lang);
			guardar();
		}),
	);
}

// Lo que falta frente a los catálogos de Scryfall.
for (const lang of lista) {
	const d = idiomas[lang];
	const sinTipo = [...supertipos, ...tiposDeCarta].filter(
		(t) => !d.tipos[t] && !Object.keys(d.tipos).some((k) => k.split(" ").includes(t)),
	);
	const todos = [...catalogos.values()].flat();
	const sinSub = todos.filter((s) => !d.subtipos[s]);
	console.error(
		`${lang}: sin tipo ${sinTipo.join(", ") || "—"}; subtipos sin traducir ${sinSub.length}/${todos.length}`,
	);
}
