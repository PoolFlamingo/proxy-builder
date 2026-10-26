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

	it("un rechazo de la mesa sale con el texto de su motivo", () => {
		const e = { code: "Rejected", params: { motivo: "SinPermiso" } };
		expect(mensajeDeError(e, t)).toBe(
			'Eso es de otro jugador: no puedes tocarlo.|{"motivo":"SinPermiso"}',
		);
		// Un motivo desconocido, con el texto general.
		const otro = { code: "Rejected", params: { motivo: "Nuevo" } };
		expect(mensajeDeError(otro, t)).toBe('La mesa no deja hacer eso ahora.|{"motivo":"Nuevo"}');
	});

	it("actualizar con la mesa abierta dice que se acabe la partida", () => {
		const e = { code: "Busy", params: { motivo: "partida" } };
		expect(mensajeDeError(e, t)).toBe(
			'Hay una partida abierta: termínala o sal de la mesa para actualizar.|{"motivo":"partida"}',
		);
		expect(mensajeDeError({ code: "Busy", params: {} }, t)).toBe(
			"Hay un trabajo en marcha; espera a que termine.|{}",
		);
	});
});
