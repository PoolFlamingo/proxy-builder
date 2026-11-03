import { describe, expect, it } from "vitest";
import {
	componer,
	conAndroid,
	ErrorDeUso,
	fallosDeRelease,
	leerArgs,
	limpiarNotas,
	nombreApk,
	notasDelActualizador,
	plantilla,
	TEXTO_FIJO,
} from "../release-notas.mjs";

describe("leerArgs", () => {
	it("lee el tipo y las opciones sin valor", () => {
		expect(leerArgs(["patch", "--dry", "--publicar"])).toMatchObject({
			tipo: "patch",
			dry: true,
			publicarYa: true,
			sinNotas: false,
		});
	});

	it("el valor de --notas no se toma por el tipo", () => {
		const r = leerArgs(["--notas", "Arreglos", "minor"]);
		expect(r.tipo).toBe("minor");
		expect(r.notas).toBe("Arreglos");
		expect(leerArgs(["1.4.0", "--notas-fichero", "notas.md"]).notasFichero).toBe("notas.md");
	});

	it("falla con opciones que no conoce, sin valor o incompatibles", () => {
		expect(() => leerArgs(["patch", "--nota", "x"])).toThrow(ErrorDeUso);
		expect(() => leerArgs(["patch", "--notas"])).toThrow(ErrorDeUso);
		expect(() => leerArgs(["patch", "--notas", "--dry"])).toThrow(ErrorDeUso);
		expect(() => leerArgs(["patch", "--notas", "x", "--sin-notas"])).toThrow(ErrorDeUso);
		expect(() => leerArgs(["patch", "minor"])).toThrow(ErrorDeUso);
	});
});

describe("notas", () => {
	it("la plantilla entera es un comentario: limpia queda vacía", () => {
		const p = plantilla("2.3.0", ["abc123 Arreglo", "def456 Otra cosa"]);
		expect(p).toContain("abc123 Arreglo");
		expect(limpiarNotas(p)).toBe("");
		expect(limpiarNotas(`${p}## Novedades\n\n- OCR mejor\n`)).toBe("## Novedades\n\n- OCR mejor");
	});

	it("el texto fijo va debajo de las notas", () => {
		expect(componer("Hola")).toBe(`Hola\n\n${TEXTO_FIJO}\n`);
		expect(componer("<!-- nada -->")).toBe(`${TEXTO_FIJO}\n`);
	});
});

describe("latest.json", () => {
	const json = JSON.stringify({
		version: "2.4.0",
		notes: TEXTO_FIJO,
		pub_date: "2026-10-20T10:00:00Z",
		platforms: { "linux-x86_64-deb": { signature: "x", url: "u" } },
	});

	it("cambia solo las notas", () => {
		const r = JSON.parse(notasDelActualizador(json, "2.4.0", "<!-- c -->\n- OCR mejor\n"));
		expect(r.notes).toBe("- OCR mejor");
		expect(r.platforms["linux-x86_64-deb"]).toEqual({ signature: "x", url: "u" });
		expect(r.pub_date).toBe("2026-10-20T10:00:00Z");
	});

	it("sin notas las deja vacías", () => {
		expect(JSON.parse(notasDelActualizador(json, "2.4.0", null)).notes).toBe("");
	});

	it("no toca el de otra versión", () => {
		expect(() => notasDelActualizador(json, "2.5.0", "x")).toThrow();
	});
});

describe("entrada de Android en latest.json", () => {
	const json = JSON.stringify({
		version: "2.4.0",
		notes: "x",
		pub_date: "2026-10-20T10:00:00Z",
		platforms: { "linux-x86_64-deb": { signature: "x", url: "u" } },
	});
	const firma = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZQ==\n";

	it("añade android-aarch64 con la URL del APK y la firma, sin tocar lo demás", () => {
		const r = JSON.parse(conAndroid(json, "2.4.0", "PoolFlamingo/proxy-builder", firma));
		expect(r.platforms["android-aarch64"]).toEqual({
			signature: "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZQ==",
			url: "https://github.com/PoolFlamingo/proxy-builder/releases/download/v2.4.0/Proxy-Builder_2.4.0_android-arm64.apk",
		});
		expect(r.platforms["linux-x86_64-deb"]).toEqual({ signature: "x", url: "u" });
		expect(r.notes).toBe("x");
		expect(r.pub_date).toBe("2026-10-20T10:00:00Z");
	});

	it("repetirlo no duplica nada", () => {
		const una = conAndroid(json, "2.4.0", "a/b", firma);
		expect(conAndroid(una, "2.4.0", "a/b", firma)).toBe(una);
	});

	it("falla con otra versión o sin firma", () => {
		expect(() => conAndroid(json, "2.5.0", "a/b", firma)).toThrow();
		expect(() => conAndroid(json, "2.4.0", "a/b", "")).toThrow();
		expect(() => conAndroid(json, "2.4.0", "a/b", "no es base64!")).toThrow();
	});

	it("el nombre del APK lleva la versión y arm64", () => {
		expect(nombreApk("3.2.4")).toBe("Proxy-Builder_3.2.4_android-arm64.apk");
	});
});

describe("qué ha fallado en release.yml", () => {
	const build = (n, conclusion) => ({ name: `build (${n})`, conclusion });

	it("solo Android: el escritorio sigue", () => {
		expect(
			fallosDeRelease([
				build("ubuntu-22.04, ", "success"),
				build("windows-latest, ", "success"),
				{ name: "android", conclusion: "failure" },
			]),
		).toEqual({ escritorio: [], android: true });
	});

	it("un trabajo de escritorio: se dice cuál, y el Android saltado no cuenta", () => {
		expect(
			fallosDeRelease([
				build("ubuntu-22.04, ", "success"),
				build("windows-latest, ", "failure"),
				build("macos-latest, --target x86_64-apple-darwin", "cancelled"),
				{ name: "android", conclusion: "skipped" },
			]),
		).toEqual({
			escritorio: [
				"build (windows-latest, )",
				"build (macos-latest, --target x86_64-apple-darwin)",
			],
			android: false,
		});
	});

	it("todo bien o sin datos: nada", () => {
		expect(
			fallosDeRelease([build("a", "success"), { name: "android", conclusion: "success" }]),
		).toEqual({ escritorio: [], android: false });
		expect(fallosDeRelease(undefined)).toEqual({ escritorio: [], android: false });
	});
});
