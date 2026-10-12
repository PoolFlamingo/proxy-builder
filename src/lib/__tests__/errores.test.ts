import { describe, expect, it } from "vitest";
import { esAppError, mensajeDeError } from "@/lib/errores";

const t = (es: string, vars?: Record<string, string | number>) =>
	vars ? `${es}|${JSON.stringify(vars)}` : es;

describe("mensajeDeError", () => {
	it("traduce un AppError por su código", () => {
		const e = { code: "Cancelled", params: {} };
		expect(esAppError(e)).toBe(true);
		expect(mensajeDeError(e, t)).toBe("Cancelado.|{}");
	});

	it("no confunde un objeto cualquiera con un AppError", () => {
		expect(esAppError({ code: "NoExiste" })).toBe(false);
		expect(esAppError(null)).toBe(false);
	});

	it("deja pasar un Error normal y cualquier otra cosa", () => {
		expect(mensajeDeError(new Error("boom"), t)).toBe("boom");
		expect(mensajeDeError("texto", t)).toBe("texto");
	});
});
