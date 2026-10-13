#!/usr/bin/env node
// Saca una versión nueva de punta a punta.
//
//   pnpm release patch               # 1.0.2 → 1.0.3
//   pnpm release minor               # 1.0.2 → 1.1.0
//   pnpm release major               # 1.0.2 → 2.0.0
//   pnpm release 1.4.0               # versión exacta
//   pnpm release patch --dry         # solo comprueba y enseña qué haría
//   pnpm release patch --publicar    # publica el borrador sin preguntar
//
// Pasos: comprueba que `main` está limpia y al día con origin → sube la versión
// en los cuatro sitios (package.json, tauri.conf.json, Cargo.toml, Cargo.lock)
// → commit «Versión X» y push → espera a la CI de ese commit → lanza el
// workflow `release` (instaladores de Linux, Windows y macOS en un borrador) y
// espera → pregunta si publicar el borrador. Publicarlo crea la etiqueta `vX`.
//
// Necesita `git` y `gh` (con la sesión iniciada: `gh auth login`).

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { createInterface } from "node:readline/promises";

const RAIZ = new URL("..", import.meta.url).pathname;
const RAMA = "main";
const PAQUETE = join(RAIZ, "package.json");
const TAURI = join(RAIZ, "src-tauri/tauri.conf.json");
const CARGO = join(RAIZ, "src-tauri/Cargo.toml");
const BLOQUEO = join(RAIZ, "src-tauri/Cargo.lock");
const SEMVER = /^\d+\.\d+\.\d+$/;

const args = process.argv.slice(2);
const dry = args.includes("--dry");
const publicarYa = args.includes("--publicar");
const tipo = args.find((a) => !a.startsWith("--"));

class Fallo extends Error {}

function ejecutar(cmd, params, { silencioso = true } = {}) {
	try {
		return execFileSync(cmd, params, {
			cwd: RAIZ,
			encoding: "utf8",
			stdio: ["ignore", "pipe", silencioso ? "pipe" : "inherit"],
		}).trim();
	} catch (e) {
		const detalle = `${e.stderr ?? ""}${e.stdout ?? ""}`.trim();
		throw new Fallo(`«${cmd} ${params.join(" ")}» falló${detalle ? `:\n${detalle}` : "."}`);
	}
}
const git = (...p) => ejecutar("git", p);
const gh = (...p) => ejecutar("gh", p);

const paso = (texto) => console.log(`\n▸ ${texto}`);

// En --dry un problema se avisa y se sigue; si no, se corta.
function problema(texto) {
	if (!dry) throw new Fallo(texto);
	console.log(`  ⚠ ${texto}`);
}

function versionDe(texto, patron, donde) {
	const m = texto.match(patron);
	if (!m) throw new Fallo(`No encuentro la versión en ${donde}.`);
	return m[1];
}

const leer = (f) => readFileSync(f, "utf8");
const PATRON_JSON = /"version":\s*"([^"]+)"/;
const PATRON_CARGO = /^version = "([^"]+)"/m;
const PATRON_BLOQUEO = /(name = "proxy-builder"\nversion = ")([^"]+)(")/;

function versionActual() {
	const versiones = {
		"package.json": versionDe(leer(PAQUETE), PATRON_JSON, "package.json"),
		"tauri.conf.json": versionDe(leer(TAURI), PATRON_JSON, "tauri.conf.json"),
		"Cargo.toml": versionDe(leer(CARGO), PATRON_CARGO, "Cargo.toml"),
		"Cargo.lock": versionDe(
			leer(BLOQUEO),
			/name = "proxy-builder"\nversion = "([^"]+)"/,
			"Cargo.lock",
		),
	};
	if (new Set(Object.values(versiones)).size !== 1) {
		throw new Fallo(
			`Las versiones no coinciden: ${Object.entries(versiones)
				.map(([f, v]) => `${f} ${v}`)
				.join(", ")}. Arréglalo a mano antes de sacar una versión.`,
		);
	}
	return versiones["package.json"];
}

function siguiente(actual, tipo) {
	if (SEMVER.test(tipo)) return tipo;
	const [ma, mi, pa] = actual.split(".").map(Number);
	if (tipo === "major") return `${ma + 1}.0.0`;
	if (tipo === "minor") return `${ma}.${mi + 1}.0`;
	if (tipo === "patch") return `${ma}.${mi}.${pa + 1}`;
	throw new Fallo("Uso: pnpm release <patch|minor|major|X.Y.Z> [--dry] [--publicar]");
}

function subirVersion(nueva) {
	const cambiar = (fichero, patron, reemplazo) => {
		const antes = leer(fichero);
		const despues = antes.replace(patron, reemplazo);
		if (antes === despues) throw new Fallo(`No pude cambiar la versión en ${fichero}.`);
		writeFileSync(fichero, despues);
	};
	cambiar(PAQUETE, PATRON_JSON, `"version": "${nueva}"`);
	cambiar(TAURI, PATRON_JSON, `"version": "${nueva}"`);
	cambiar(CARGO, PATRON_CARGO, `version = "${nueva}"`);
	cambiar(BLOQUEO, PATRON_BLOQUEO, (_todo, antes, _vieja, despues) => `${antes}${nueva}${despues}`);
}

async function preguntar(texto) {
	const rl = createInterface({ input: process.stdin, output: process.stdout });
	try {
		return (await rl.question(texto)).trim().toLowerCase();
	} finally {
		rl.close();
	}
}

const dormir = (ms) => new Promise((r) => setTimeout(r, ms));

// Espera a que aparezca una ejecución y la sigue hasta el final.
async function esperarEjecucion(flujo, sha, desde, extra = []) {
	let id;
	for (let i = 0; i < 30 && !id; i++) {
		const lista = JSON.parse(
			gh(
				"run",
				"list",
				"--workflow",
				flujo,
				...extra,
				"--json",
				"databaseId,headSha,createdAt",
				"--limit",
				"10",
			),
		);
		id = lista.find((r) => r.headSha === sha && new Date(r.createdAt) >= desde)?.databaseId;
		if (!id) await dormir(4000);
	}
	if (!id) throw new Fallo(`No aparece la ejecución de «${flujo}» para ${sha.slice(0, 7)}.`);
	console.log(`  https://github.com/${repo()}/actions/runs/${id}`);
	try {
		ejecutar("gh", ["run", "watch", String(id), "--exit-status"]);
	} catch {
		throw new Fallo(`«${flujo}» ha fallado. Míralo en la URL de arriba.`);
	}
}

let _repo;
function repo() {
	_repo ??= gh("repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner");
	return _repo;
}

async function main() {
	if (!tipo) throw new Fallo("Uso: pnpm release <patch|minor|major|X.Y.Z> [--dry] [--publicar]");

	paso("Comprobaciones");
	try {
		gh("auth", "status");
	} catch {
		throw new Fallo("`gh` no tiene sesión: ejecuta `gh auth login`.");
	}
	const actual = versionActual();
	const nueva = siguiente(actual, tipo);
	if (!SEMVER.test(nueva)) throw new Fallo(`«${nueva}» no es una versión X.Y.Z.`);
	if (nueva === actual) throw new Fallo(`Ya estás en la ${actual}.`);
	const etiqueta = `v${nueva}`;
	console.log(`  ${actual} → ${nueva}`);

	if (git("branch", "--show-current") !== RAMA) problema(`Hay que estar en «${RAMA}».`);
	if (git("status", "--porcelain")) problema("Hay cambios sin commitear.");
	git("fetch", "origin", "--tags");
	if (git("rev-list", "--count", `HEAD..origin/${RAMA}`) !== "0") {
		problema(`origin/${RAMA} tiene commits que no tienes: haz pull.`);
	}
	if (git("rev-list", "--count", `origin/${RAMA}..HEAD`) !== "0") {
		problema(`Tienes commits sin subir: súbelos antes (o van dentro de la ${nueva}).`);
	}
	if (git("tag", "--list", etiqueta)) problema(`La etiqueta ${etiqueta} ya existe.`);
	if (
		gh("release", "list", "--json", "tagName", "--jq", ".[].tagName").split("\n").includes(etiqueta)
	) {
		problema(`Ya hay un release ${etiqueta} en GitHub.`);
	}

	if (dry) {
		console.log(`\nDry run: subiría la versión a ${nueva}, commit, push, CI, release y publicar.`);
		return;
	}

	paso(`Subiendo la versión a ${nueva}`);
	subirVersion(nueva);
	git(
		"add",
		"package.json",
		"src-tauri/tauri.conf.json",
		"src-tauri/Cargo.toml",
		"src-tauri/Cargo.lock",
	);
	git("commit", "-m", `Versión ${nueva}`);
	const sha = git("rev-parse", "HEAD");
	git("push", "origin", RAMA);
	console.log(`  commit ${sha.slice(0, 7)} subido`);

	paso("Esperando a la CI");
	await esperarEjecucion("ci.yml", sha, new Date(Date.now() - 60_000));

	paso("Compilando los instaladores (unos 10 minutos)");
	const desde = new Date(Date.now() - 5_000);
	gh("workflow", "run", "release.yml", "--ref", RAMA);
	await esperarEjecucion("release.yml", sha, desde, ["--event", "workflow_dispatch"]);

	paso("Borrador listo");
	const borrador = JSON.parse(
		gh("release", "view", etiqueta, "--json", "isDraft,targetCommitish,assets,url"),
	);
	if (!borrador.isDraft || borrador.targetCommitish !== sha) {
		throw new Fallo(`El release ${etiqueta} no es el borrador esperado (${borrador.url}).`);
	}
	console.log(`  ${borrador.assets.length} ficheros en ${borrador.url}`);

	const publicar =
		publicarYa ||
		["s", "si", "sí", "y", "yes"].includes(await preguntar("\n¿Publicar ahora? [s/N] "));
	if (!publicar) {
		console.log(
			`\nQueda en borrador. Para publicarlo: gh release edit ${etiqueta} --draft=false --latest`,
		);
		return;
	}
	gh("release", "edit", etiqueta, "--draft=false", "--latest");
	console.log(`\n✔ Publicado: https://github.com/${repo()}/releases/tag/${etiqueta}`);
}

main().catch((e) => {
	console.error(e instanceof Fallo ? `\n✖ ${e.message}` : e);
	process.exit(1);
});
