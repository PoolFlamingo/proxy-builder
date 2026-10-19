// Lo que no necesita ni git ni gh de `release.mjs`: leer las opciones y
// preparar el texto del release. Aparte para poder probarlo (vitest).

/** El texto que el workflow `release.yml` pone siempre; va al final de las notas. */
export const TEXTO_FIJO = "Instaladores de Proxy Builder. Sin firma de código: ver el README.";

export const USO =
	"Uso: pnpm release <patch|minor|major|X.Y.Z> [--dry] [--publicar]\n" +
	'                  [--notas "texto" | --notas-fichero RUTA | --sin-notas]';

export class ErrorDeUso extends Error {}

/** Las opciones de la línea de órdenes. Las que llevan valor lo toman del siguiente argumento. */
export function leerArgs(argv) {
	const r = {
		tipo: undefined,
		dry: false,
		publicarYa: false,
		notas: undefined,
		notasFichero: undefined,
		sinNotas: false,
	};
	for (let i = 0; i < argv.length; i++) {
		const a = argv[i];
		const valor = () => {
			const v = argv[++i];
			if (v === undefined || v.startsWith("--")) throw new ErrorDeUso(`Falta el valor de ${a}.`);
			return v;
		};
		if (a === "--dry") r.dry = true;
		else if (a === "--publicar") r.publicarYa = true;
		else if (a === "--sin-notas") r.sinNotas = true;
		else if (a === "--notas") r.notas = valor();
		else if (a === "--notas-fichero") r.notasFichero = valor();
		else if (a.startsWith("--")) throw new ErrorDeUso(`No conozco la opción ${a}.`);
		else if (r.tipo === undefined) r.tipo = a;
		else throw new ErrorDeUso(`Sobra «${a}».`);
	}
	const fuentes = [r.notas !== undefined, r.notasFichero !== undefined, r.sinNotas];
	if (fuentes.filter(Boolean).length > 1) {
		throw new ErrorDeUso("Elige solo una: --notas, --notas-fichero o --sin-notas.");
	}
	return r;
}

/** Quita los comentarios `<!-- … -->` (los de la plantilla) y los espacios de los extremos. */
export function limpiarNotas(texto) {
	return texto
		.replace(/<!--[\s\S]*?-->/g, "")
		.replace(/\n{3,}/g, "\n\n")
		.trim();
}

/** Lo que se abre en el editor cuando no se pasan las notas. */
export function plantilla(version, commits) {
	const lista = commits.length
		? commits.map((c) => `  ${c}`).join("\n")
		: "  (ninguno desde la última etiqueta)";
	return [
		`<!-- Notas del release v${version}. Escribe debajo, en Markdown.`,
		"     Las líneas entre estos comentarios no salen. Guarda y cierra para seguir;",
		"     si lo dejas vacío, se corta sin tocar nada.",
		"",
		"     Commits desde la última versión:",
		lista,
		"-->",
		"",
		"",
	].join("\n");
}

/** Las notas que se ponen en el release: las escritas y, debajo, el texto fijo. */
export function componer(notas) {
	const limpias = limpiarNotas(notas);
	return limpias ? `${limpias}\n\n${TEXTO_FIJO}\n` : `${TEXTO_FIJO}\n`;
}

/**
 * El `latest.json` del actualizador con las notas de verdad: el workflow le
 * pone el texto fijo, que en la app no dice nada. Sin notas, vacío. Falla si
 * no es el `latest.json` de la versión esperada.
 */
export function notasDelActualizador(json, version, notas) {
	const datos = JSON.parse(json);
	if (datos.version !== version || typeof datos.platforms !== "object" || !datos.platforms) {
		throw new Error(`latest.json no es el de la ${version}.`);
	}
	datos.notes = notas === null ? "" : limpiarNotas(notas);
	return `${JSON.stringify(datos, null, 2)}\n`;
}
