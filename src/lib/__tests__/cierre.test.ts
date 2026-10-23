import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/ipc", () => ({ call: vi.fn(), on: vi.fn() }));

import { hayPendientes, pendientes } from "@/lib/cierre";
import { registrarDialogo } from "@/lib/dialogos-abiertos";
import { useActualizacion } from "@/stores/actualizacion-store";
import { useEdicion } from "@/stores/edicion-store";
import { useTrabajo } from "@/stores/trabajo-store";

afterEach(() => {
	useTrabajo.getState().terminar();
	useEdicion.getState().poner(null);
	useActualizacion.setState({ paso: "inactiva" });
});

describe("pendientes al cerrar la app", () => {
	it("sin nada en marcha ni sin guardar, se cierra sin preguntar", () => {
		const p = pendientes();
		expect(p).toEqual({
			trabajo: null,
			actualizando: false,
			dialogoEnCurso: false,
			dialogoSinGuardar: false,
			edicion: false,
			deck: false,
		});
		expect(hayPendientes(p)).toBe(false);
	});

	it("un trabajo largo en marcha pregunta, con su fase (también sin texto)", () => {
		useTrabajo.getState().empezar("exportar", "Preparando imágenes…");
		expect(pendientes().trabajo).toBe("Preparando imágenes…");
		expect(hayPendientes(pendientes())).toBe(true);
		useTrabajo.getState().terminar();
		useTrabajo.getState().empezar("resolver", "");
		expect(hayPendientes(pendientes())).toBe(true);
	});

	it("la carta que se diseña sin guardar pregunta, y guardar la deja de contar", () => {
		useEdicion.getState().poner({ guardar: async () => true });
		expect(pendientes().edicion).toBe(true);
		expect(hayPendientes(pendientes())).toBe(true);
		useEdicion.getState().poner(null);
		expect(pendientes().edicion).toBe(false);
	});

	it("una actualización bajando o instalando pregunta; buscar o tenerla lista, no", () => {
		for (const paso of ["bajando", "instalando"] as const) {
			useActualizacion.setState({ paso });
			expect(pendientes().actualizando).toBe(true);
		}
		for (const paso of ["inactiva", "buscando", "al-dia", "disponible", "lista"] as const) {
			useActualizacion.setState({ paso });
			expect(pendientes().actualizando).toBe(false);
		}
	});

	it("los diálogos abiertos dicen si tienen un proceso o algo sin guardar", () => {
		let estado = { enCurso: false, sinGuardar: false };
		const quitar = registrarDialogo(() => estado);
		expect(hayPendientes(pendientes())).toBe(false);
		estado = { enCurso: true, sinGuardar: false };
		expect(pendientes()).toMatchObject({ dialogoEnCurso: true, dialogoSinGuardar: false });
		estado = { enCurso: false, sinGuardar: true };
		expect(pendientes()).toMatchObject({ dialogoEnCurso: false, dialogoSinGuardar: true });
		// Un diálogo que se cierra ya no cuenta.
		quitar();
		expect(hayPendientes(pendientes())).toBe(false);
	});

	it("varios diálogos se juntan: basta con que uno lo diga", () => {
		const quitarA = registrarDialogo(() => ({ enCurso: false, sinGuardar: false }));
		const quitarB = registrarDialogo(() => ({ enCurso: true, sinGuardar: false }));
		expect(pendientes().dialogoEnCurso).toBe(true);
		quitarB();
		expect(pendientes().dialogoEnCurso).toBe(false);
		quitarA();
	});

	it("un deck que no se pudo guardar pregunta", () => {
		expect(pendientes(true).deck).toBe(true);
		expect(hayPendientes(pendientes(true))).toBe(true);
		expect(hayPendientes(pendientes(false))).toBe(false);
	});
});
