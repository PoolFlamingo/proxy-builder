import { describe, expect, it } from "vitest";
import { CargaUltima } from "@/lib/imagen-viva";

/** Un cargador de mentira: guarda las cargas y las termina cuando se le dice. */
function cargadorManual() {
	const pendientes: { url: string; listo: (ok: boolean) => void }[] = [];
	return {
		pendientes,
		cargar: (url: string, listo: (ok: boolean) => void) => pendientes.push({ url, listo }),
		acabar(ok = true) {
			const p = pendientes.shift();
			p?.listo(ok);
			return p?.url;
		},
	};
}

describe("CargaUltima", () => {
	it("carga de una en una y, de las pedidas mientras tanto, solo la última", () => {
		const c = cargadorManual();
		const vistas: string[] = [];
		const carga = new CargaUltima(c.cargar, (u) => vistas.push(u));
		carga.pedir("a");
		carga.pedir("b");
		carga.pedir("c");
		carga.pedir("d");
		expect(c.pendientes.map((p) => p.url)).toEqual(["a"]);
		c.acabar();
		// «b» y «c» se saltan: tras «a» va directamente «d».
		expect(c.pendientes.map((p) => p.url)).toEqual(["d"]);
		c.acabar();
		expect(vistas).toEqual(["a", "d"]);
		expect(carga.ocupado).toBe(false);
	});

	it("una imagen que falla no se muestra, pero se sigue con la siguiente", () => {
		const c = cargadorManual();
		const vistas: string[] = [];
		const carga = new CargaUltima(c.cargar, (u) => vistas.push(u));
		carga.pedir("mala");
		carga.pedir("buena");
		c.acabar(false);
		c.acabar(true);
		expect(vistas).toEqual(["buena"]);
	});

	it("pedir la misma que está cargando no la repite", () => {
		const c = cargadorManual();
		const carga = new CargaUltima(c.cargar, () => {});
		carga.pedir("a");
		carga.pedir("b");
		carga.pedir("a");
		c.acabar();
		expect(c.pendientes).toHaveLength(0);
	});

	it("avisa de cuándo está ocupada", () => {
		const c = cargadorManual();
		const estados: boolean[] = [];
		const carga = new CargaUltima(
			c.cargar,
			() => {},
			(o) => estados.push(o),
		);
		carga.pedir("a");
		carga.pedir("b");
		c.acabar();
		c.acabar();
		// Ocupada de «a» a «b» sin pasar por libre entre medias.
		expect(estados).toEqual([true, false]);
	});

	it("un cargador que termina enseguida (de caché) también funciona", () => {
		const vistas: string[] = [];
		const carga = new CargaUltima(
			(_u, listo) => listo(true),
			(u) => vistas.push(u),
		);
		carga.pedir("a");
		carga.pedir("b");
		expect(vistas).toEqual(["a", "b"]);
		expect(carga.ocupado).toBe(false);
	});
});
