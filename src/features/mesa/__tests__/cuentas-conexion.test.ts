import { describe, expect, it } from "vitest";
import { aspectoDeConexion } from "../red/cuentas-conexion";

describe("cómo se enseña la conexión al volver de segundo plano", () => {
	it("perdida justo tras volver se enseña como «volviendo», no como un error", () => {
		expect(aspectoDeConexion("perdida", 20_000, 5_000)).toBe("volviendo");
	});
	it("pasado el plazo vuelve a ser un error de verdad", () => {
		expect(aspectoDeConexion("perdida", 20_000, 20_000)).toBe("perdida");
		expect(aspectoDeConexion("perdida", 0, 5_000)).toBe("perdida");
	});
	it("lo demás se enseña como es", () => {
		expect(aspectoDeConexion("conectada", 20_000, 5_000)).toBe("conectada");
		expect(aspectoDeConexion("reconectando", 20_000, 5_000)).toBe("reconectando");
		expect(aspectoDeConexion("buscando", 20_000, 5_000)).toBe("buscando");
	});
});
