import { describe, expect, it } from "vitest";
import type { Accion } from "@/bindings/Accion";
import type { Evento } from "@/bindings/Evento";
import type { Nota } from "@/bindings/Nota";
import { repartirNotas } from "../cuentas-notas";
import { visibleEnRegistro } from "../cuentas-registro";
import { textoDeEvento } from "../textos-eventos";

const t = (es: string, vars?: Record<string, string | number>) =>
	es.replace(/\{(\w+)\}/g, (_, k: string) => String(vars?.[k] ?? `{${k}}`));

const nota = (id: number, autor: number, extra: Partial<Nota> = {}): Nota => ({
	id,
	autor,
	texto: `n${id}`,
	temporal: false,
	privada: false,
	...extra,
});

const evento = (accion: Accion): Evento => ({
	seq: 1,
	tMs: 0,
	actor: { tipo: "jugador", id: 0 },
	cambio: { tipo: "accion", accion },
});

describe("repartirNotas", () => {
	it("separa las mías de las de los demás, por asiento y por orden de creación", () => {
		const notas = [nota(4, 2), nota(1, 0), nota(3, 1), nota(2, 0), nota(5, 1)];
		const r = repartirNotas(notas, 0, [0, 1, 2]);
		expect(r.mias.map((n) => n.id)).toEqual([1, 2]);
		expect(r.ajenas.map((a) => [a.autor, a.notas.map((n) => n.id)])).toEqual([
			[1, [3, 5]],
			[2, [4]],
		]);
	});

	it("un espectador no tiene notas propias y los asientos sin notas no salen", () => {
		const r = repartirNotas([nota(1, 1)], null, [0, 1, 2]);
		expect(r.mias).toEqual([]);
		expect(r.ajenas.map((a) => a.autor)).toEqual([1]);
	});
});

describe("notas en el registro", () => {
	it("una privada no sale, ni se nombra al borrar", () => {
		const privada = { que: "ponerNota", texto: "secreto", temporal: false, privada: true } as const;
		expect(visibleEnRegistro(evento(privada))).toBe(false);
		expect(
			visibleEnRegistro(
				evento({ que: "editarNota", id: 1, texto: "x", temporal: false, privada: true }),
			),
		).toBe(false);
		expect(visibleEnRegistro(evento({ que: "quitarNota", id: 1 }))).toBe(false);
	});

	it("una pública sale con su texto", () => {
		const e = evento({ que: "ponerNota", texto: "3 de maná", temporal: true, privada: false });
		expect(visibleEnRegistro(e)).toBe(true);
		expect(textoDeEvento(e, { cartas: {}, jugadores: {} }, t)).toBe(
			"? apunta una nota: «3 de maná»",
		);
	});

	it("lo que no es una nota sigue saliendo", () => {
		expect(visibleEnRegistro(evento({ que: "barajar" }))).toBe(true);
	});
});
