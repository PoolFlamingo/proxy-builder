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

/** El APK de Android que sube el trabajo `android` de `release.yml` (solo arm64). */
export function nombreApk(version) {
	return `Proxy-Builder_${version}_android-arm64.apk`;
}

/** La clave de `latest.json` para el APK: `android-<arquitectura>` (`.target(…)` de `actualizar.rs`). */
export const OBJETIVO_ANDROID = "android-aarch64";

/**
 * Añade al `latest.json` del actualizador la entrada del APK de Android
 * (`android-aarch64`) con su URL y la firma del actualizador (el contenido de
 * su `.sig`). El trabajo `android` del workflow no toca `latest.json` (los de
 * escritorio lo reescriben en paralelo): lo hace este script al final. Lo que
 * ya hay (escritorio, notas, fecha) no se toca. Falla si no es el
 * `latest.json` de la versión esperada o si la firma está vacía.
 */
export function conAndroid(json, version, repo, firma) {
	const datos = JSON.parse(json);
	if (datos.version !== version || typeof datos.platforms !== "object" || !datos.platforms) {
		throw new Error(`latest.json no es el de la ${version}.`);
	}
	const signature = String(firma ?? "").trim();
	if (!/^[A-Za-z0-9+/=]{16,}$/.test(signature)) {
		throw new Error(`La firma del APK (${nombreApk(version)}.sig) está vacía o no es base64.`);
	}
	datos.platforms[OBJETIVO_ANDROID] = {
		signature,
		url: `https://github.com/${repo}/releases/download/v${version}/${nombreApk(version)}`,
	};
	return `${JSON.stringify(datos, null, 2)}\n`;
}

/** Cómo acaba un trabajo que cuenta como fallo en `gh run view --json jobs`. */
const FALLIDOS = new Set([
	"failure",
	"cancelled",
	"timed_out",
	"startup_failure",
	"action_required",
]);

/**
 * Qué trabajos de una ejecución de `release.yml` han fallado (los `jobs` de
 * `gh run view <id> --json jobs`): los de escritorio (`build (…)`) por nombre
 * y si falló el de Android (`android`). Uno saltado (`skipped`, p. ej.
 * `android` porque falló un `build`) no cuenta como fallo.
 */
export function fallosDeRelease(jobs) {
	const fallidos = (Array.isArray(jobs) ? jobs : []).filter((j) =>
		FALLIDOS.has(String(j?.conclusion ?? "").toLowerCase()),
	);
	const esAndroid = (j) => String(j?.name ?? "").trim() === "android";
	return {
		escritorio: fallidos.filter((j) => !esAndroid(j)).map((j) => String(j?.name ?? "?")),
		android: fallidos.some(esAndroid),
	};
}
