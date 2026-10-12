#!/usr/bin/env node
/**
 * Falla si algún `.tsx` de `src/` pone `title` a una ETIQUETA HTML (`button`,
 * `p`, `span`, `div`…). El texto que se enseña al pasar el ratón va SIEMPRE
 * con el `Tooltip` de `components/ui/tooltip.tsx`, o con `WithTooltip`.
 *
 * 🛑 Por qué es un error y no un aviso: en la ventana de Linux el `title`
 * nativo lo pinta GTK con los colores del sistema, y se ve como si fuera de
 * otra aplicación. Se barrieron todos a mano y al día siguiente ya había
 * cuatro nuevos (Pablo, 26/09/2026). `TooltipsNativos` los convierte en
 * ejecución si alguno se cuela igualmente; esto es para que no se cuelen.
 *
 * Mira las etiquetas en minúscula y los componentes de `PASAN_AL_DOM`, que
 * dejan el `title` en su etiqueta: `<SettingCard title=…>` o
 * `<DialogoConfirmar titulo=…>` son props de un componente nuestro y no
 * pintan nada nativo. Excepción: `iframe`/`frame`, donde `title` es el nombre
 * accesible del marco, no un tooltip.
 *
 * Uso: `node scripts/sin-title-nativo.mjs` (va en `pnpm check` y en el
 * pre-commit).
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const RAIZ = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const EXCEPCIONES = new Set(["iframe", "frame"]);
/**
 * Componentes nuestros que pasan sus props tal cual a una etiqueta HTML: un
 * `title=` en ellos acaba igual de nativo. `<TableCell title=…>` en el visor de
 * CSV se coló por aquí (revisión del 01–05/10/2026), porque la primera versión
 * solo miraba etiquetas en minúscula.
 */
const PASAN_AL_DOM = new Set([
	"Table",
	"TableHeader",
	"TableBody",
	"TableRow",
	"TableHead",
	"TableCell",
	"TableCaption",
	"Badge",
	"Label",
	// El de la casa pasa sus props al `<button>`: un `title` en él llegaba
	// nativo y este script no lo veía (auditoría del 06/10/2026, COM-D-11).
	"Button",
]);

function* ficheros(dir) {
	for (const nombre of readdirSync(dir)) {
		const ruta = join(dir, nombre);
		if (statSync(ruta).isDirectory()) yield* ficheros(ruta);
		else if (ruta.endsWith(".tsx")) yield ruta;
	}
}

/**
 * Los atributos de primer nivel de cada etiqueta en minúscula: lo que hay
 * entre `<tag` y su `>`, saltando lo que va entre llaves (una expresión puede
 * llevar `=>` o JSX anidado) y entre comillas.
 */
function* etiquetas(src) {
	const inicio = /<([A-Za-z][A-Za-z0-9-]*)(?=[\s/>])/g;
	for (let m = inicio.exec(src); m; m = inicio.exec(src)) {
		let prof = 0;
		let comilla = null;
		let plano = "";
		let j = m.index + m[0].length;
		for (; j < src.length; j++) {
			const c = src[j];
			if (comilla) {
				if (c === comilla) comilla = null;
				if (prof === 0) plano += c;
				continue;
			}
			if (c === "{") prof++;
			else if (c === "}") prof--;
			else if (prof === 0 && (c === '"' || c === "'")) comilla = c;
			else if (prof === 0 && c === ">") break;
			if (prof === 0 && c !== "}") plano += c;
		}
		yield { tag: m[1], indice: m.index, atributos: plano };
	}
}

const fallos = [];
for (const fichero of ficheros(join(RAIZ, "src"))) {
	const src = readFileSync(fichero, "utf8");
	for (const { tag, indice, atributos } of etiquetas(src)) {
		if (EXCEPCIONES.has(tag)) continue;
		if (/^[A-Z]/.test(tag) && !PASAN_AL_DOM.has(tag)) continue;
		if (!/(^|\s)title\s*=/.test(atributos)) continue;
		const linea = src.slice(0, indice).split("\n").length;
		fallos.push(`${relative(RAIZ, fichero)}:${linea}  <${tag} title=…>`);
	}
}

if (fallos.length > 0) {
	console.error(
		`\x1b[31m✖ ${fallos.length} \`title\` nativo(s). El texto de «pasar el ratón» va con <Tooltip> (components/ui/tooltip.tsx) o <WithTooltip>, nunca con title=.\x1b[0m`,
	);
	for (const f of fallos) console.error(`   · ${f}`);
	process.exit(1);
}
console.log("\x1b[32m✓ Ningún `title` nativo en src/.\x1b[0m");
