import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ActualizacionInfo } from "@/bindings/ActualizacionInfo";

const llamadas: { cmd: string; args: Record<string, unknown> }[] = [];
let nueva: ActualizacionInfo | null = null;
let falla: string | null = null;
let alProgreso: ((p: { hecho: number; total: number }) => void) | null = null;
vi.mock("@/lib/ipc", () => ({
	call: vi.fn(async (cmd: string, args: Record<string, unknown>) => {
		llamadas.push({ cmd, args });
		if (cmd === falla) throw { code: "Network", params: {} };
		switch (cmd) {
			case "update_status":
				return { soportada: true, actual: "2.3.0" };
			case "update_check":
				return nueva;
			case "update_download":
				alProgreso?.({ hecho: 50, total: 200 });
				return null;
			case "update_install":
				return null;
		}
		throw new Error(`orden inesperada ${cmd}`);
	}),
	on: vi.fn(async (_evento: string, f: (p: { hecho: number; total: number }) => void) => {
		alProgreso = f;
		return () => {
			alProgreso = null;
		};
	}),
}));
vi.mock("@/stores/deck-store", () => ({
	guardarAhora: vi.fn(async () => {
		llamadas.push({ cmd: "guardarAhora", args: {} });
	}),
}));

import { useActualizacion } from "@/stores/actualizacion-store";

const info = (version: string): ActualizacionInfo => ({
	version,
	actual: "2.3.0",
	notas: null,
	fecha: null,
	pidePermiso: false,
});
const ordenes = () => llamadas.map((l) => l.cmd);

describe("actualización de la app", () => {
	beforeEach(() => {
		llamadas.length = 0;
		nueva = null;
		falla = null;
		useActualizacion.setState({
			soportada: null,
			actual: null,
			paso: "inactiva",
			info: null,
			progreso: null,
			error: null,
		});
	});

	it("pregunta a Rust una sola vez si se puede", async () => {
		expect(await useActualizacion.getState().iniciar()).toBe(true);
		expect(await useActualizacion.getState().iniciar()).toBe(true);
		expect(ordenes()).toEqual(["update_status"]);
		expect(useActualizacion.getState().actual).toBe("2.3.0");
	});

	it("sin versión nueva queda al día", async () => {
		expect(await useActualizacion.getState().buscar()).toBeNull();
		expect(useActualizacion.getState().paso).toBe("al-dia");
	});

	it("busca, baja la versión encontrada e instala tras guardar el deck", async () => {
		nueva = info("2.4.0");
		const a = useActualizacion.getState();
		expect(await a.buscar()).toEqual(nueva);
		expect(useActualizacion.getState().paso).toBe("disponible");
		expect(await useActualizacion.getState().bajar()).toBe(true);
		expect(useActualizacion.getState().paso).toBe("lista");
		expect(useActualizacion.getState().progreso).toBeNull();
		await useActualizacion.getState().instalar();
		expect(ordenes()).toEqual([
			"update_check",
			"update_download",
			"guardarAhora",
			"update_install",
		]);
		expect(llamadas[1].args).toEqual({ version: "2.4.0" });
		expect(llamadas[3].args).toEqual({ version: "2.4.0" });
	});

	it("ya bajada no vuelve a buscar", async () => {
		useActualizacion.setState({ paso: "lista", info: info("2.4.0") });
		expect((await useActualizacion.getState().buscar())?.version).toBe("2.4.0");
		expect(ordenes()).toEqual([]);
	});

	it("un fallo al bajar deja reintentarlo", async () => {
		nueva = info("2.4.0");
		falla = "update_download";
		await useActualizacion.getState().buscar();
		expect(await useActualizacion.getState().bajar()).toBe(false);
		const s = useActualizacion.getState();
		expect(s.paso).toBe("disponible");
		expect(s.error).toEqual({ code: "Network", params: {} });
	});

	it("un fallo al instalar vuelve a «lista» y se lanza", async () => {
		useActualizacion.setState({ paso: "lista", info: info("2.4.0") });
		falla = "update_install";
		await expect(useActualizacion.getState().instalar()).rejects.toEqual({
			code: "Network",
			params: {},
		});
		expect(useActualizacion.getState().paso).toBe("lista");
	});

	it("sin red al buscar no inventa una versión", async () => {
		falla = "update_check";
		expect(await useActualizacion.getState().buscar()).toBeNull();
		expect(useActualizacion.getState().paso).toBe("inactiva");
	});
});
