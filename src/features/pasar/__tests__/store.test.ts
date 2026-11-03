import { beforeEach, describe, expect, it } from "vitest";
import { usePasarUi } from "../store";

describe("el diálogo de pasar decks", () => {
	beforeEach(() => usePasarUi.getState().cerrar());

	it("se abre en «Enviar» por defecto, con los decks que se le den", () => {
		usePasarUi.getState().abrirPasar();
		expect(usePasarUi.getState().dialogo).toEqual({ tipo: "pasar", pestana: "enviar", ids: [] });
		usePasarUi.getState().abrirPasar({ pestana: "recibir", ids: ["a"] });
		expect(usePasarUi.getState().dialogo).toEqual({
			tipo: "pasar",
			pestana: "recibir",
			ids: ["a"],
		});
	});

	it("solo hay uno abierto: importar un fichero cierra lo anterior", () => {
		usePasarUi.getState().abrirCompartir(["a", "b"]);
		expect(usePasarUi.getState().dialogo?.tipo).toBe("compartir");
		usePasarUi.getState().abrirImportar("content://docs/1");
		expect(usePasarUi.getState().dialogo).toEqual({ tipo: "importar", ruta: "content://docs/1" });
		usePasarUi.getState().cerrar();
		expect(usePasarUi.getState().dialogo).toBeNull();
	});
});
