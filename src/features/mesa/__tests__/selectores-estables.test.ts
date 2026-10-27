import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

/** Todos los ficheros de `src/features/mesa` que no son tests. */
function ficheros(dir: string): string[] {
	return readdirSync(dir).flatMap((f) => {
		const ruta = join(dir, f);
		if (statSync(ruta).isDirectory()) return f === "__tests__" ? [] : ficheros(ruta);
		return /\.(ts|tsx)$/.test(f) ? [ruta] : [];
	});
}

/**
 * 🛑 Un selector del store que devuelve un array u objeto **nuevo** en cada
 * lectura (`filter`, `map`, `Object.values`, un literal) hace que React entre en
 * bucle («Maximum update depth exceeded») y la pantalla caiga. Se devuelve lo
 * que ya está en el store (o `NINGUNA`, o `useShallow`) y se calcula fuera.
 */
const SELECTOR_INESTABLE =
	/use(?:Mesa|Pantalla|Vivo|Ui|Red)\(\s*\(?\w*\)?\s*=>\s*(?:\[|Object\.(?:values|entries|keys)\(|[\w.?]+\??\.(?:filter|map|slice|flatMap)\()/;

describe("selectores del store", () => {
	it("ninguno devuelve un array nuevo en cada lectura", () => {
		const mal = ficheros("src/features/mesa").filter((f) =>
			SELECTOR_INESTABLE.test(readFileSync(f, "utf8").replace(/\n\s*/g, " ")),
		);
		expect(mal).toEqual([]);
	});
});
