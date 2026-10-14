import { beforeEach, describe, expect, it, vi } from "vitest";

/** Oyentes de eventos de mentira: `emitir` reparte a los que siguen escuchando. */
const oyentes = new Map<string, Set<(c: unknown) => void>>();
const quitados: string[] = [];
vi.mock("@/lib/ipc", () => ({
	on: vi.fn(async (evento: string, f: (c: unknown) => void) => {
		const set = oyentes.get(evento) ?? new Set();
		set.add(f);
		oyentes.set(evento, set);
		return () => {
			set.delete(f);
			quitados.push(evento);
		};
	}),
}));

import { lanzarTrabajo } from "@/lib/trabajos";

function emitir(evento: string, carga: unknown) {
	for (const f of [...(oyentes.get(evento) ?? [])]) f(carga);
}

const escuchando = (evento: string) => oyentes.get(evento)?.size ?? 0;

describe("lanzarTrabajo", () => {
	beforeEach(() => {
		oyentes.clear();
		quitados.length = 0;
	});

	it("escucha antes de lanzar y reparte el avance de su trabajo", async () => {
		const avances: unknown[] = [];
		const t = await lanzarTrabajo(
			"export://progress",
			"export://done",
			async () => {
				expect(escuchando("export://progress")).toBe(1);
				expect(escuchando("export://done")).toBe(1);
				return 7;
			},
			(p) => avances.push(p),
		);
		expect(t.jobId).toBe(7);
		emitir("export://progress", { jobId: 7, done: 1 });
		emitir("export://progress", { jobId: 8, done: 99 });
		expect(avances).toEqual([{ jobId: 7, done: 1 }]);
		emitir("export://done", { jobId: 8, error: null });
		emitir("export://done", { jobId: 7, error: null, pages: 3 });
		await expect(t.fin).resolves.toEqual({ jobId: 7, error: null, pages: 3 });
		// Al terminar deja de escuchar los dos eventos.
		expect(escuchando("export://progress")).toBe(0);
		expect(escuchando("export://done")).toBe(0);
	});

	it("lo que llega antes de saber el id se guarda y se reparte después", async () => {
		const avances: unknown[] = [];
		const t = await lanzarTrabajo(
			"resolve://progress",
			"resolve://done",
			async () => {
				// Un trabajo tan corto que acaba antes de que el invoke devuelva.
				emitir("resolve://progress", { jobId: 3, done: 1 });
				emitir("resolve://progress", { jobId: 4, done: 1 });
				emitir("resolve://progress", { jobId: 3, done: 2 });
				emitir("resolve://done", { jobId: 4, error: null });
				emitir("resolve://done", { jobId: 3, error: null });
				return 3;
			},
			(p) => avances.push(p),
		);
		expect(avances).toEqual([
			{ jobId: 3, done: 1 },
			{ jobId: 3, done: 2 },
		]);
		await expect(t.fin).resolves.toEqual({ jobId: 3, error: null });
		expect(escuchando("resolve://done")).toBe(0);
	});

	it("si lanzar falla, deja de escuchar y propaga el error", async () => {
		await expect(
			lanzarTrabajo(
				"design://progress",
				"design://done",
				async () => {
					throw { code: "Db", params: {} };
				},
				() => {},
			),
		).rejects.toEqual({ code: "Db", params: {} });
		expect(escuchando("design://progress")).toBe(0);
		expect(escuchando("design://done")).toBe(0);
		expect(quitados.sort()).toEqual(["design://done", "design://progress"]);
	});

	it("el final con error también cumple la promesa (no la rechaza)", async () => {
		const t = await lanzarTrabajo(
			"designpng://progress",
			"designpng://done",
			async () => 1,
			() => {},
		);
		const final = { jobId: 1, files: 0, error: { code: "Cancelled", params: {} } };
		emitir("designpng://done", final);
		await expect(t.fin).resolves.toEqual(final);
	});

	it("tras el final no reparte más avance", async () => {
		const avances: unknown[] = [];
		const t = await lanzarTrabajo(
			"identify://progress",
			"identify://done",
			async () => 2,
			(p) => avances.push(p),
		);
		emitir("identify://done", { jobId: 2 });
		await t.fin;
		emitir("identify://progress", { jobId: 2, done: 5 });
		expect(avances).toEqual([]);
	});
});
