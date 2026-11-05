import { beforeEach, describe, expect, it, vi } from "vitest";

// La ventana de mentira: guarda si está a pantalla completa y apunta cada orden.
const estado = { completa: false, ordenes: [] as boolean[], falla: false };
vi.mock("@tauri-apps/api/window", () => ({
	getCurrentWindow: () => ({
		isFullscreen: async () => estado.completa,
		setFullscreen: async (si: boolean) => {
			if (estado.falla) throw new Error("sin permiso");
			// Tarda un poco, como el sistema: lo que llegue en medio tiene que esperar.
			await new Promise((r) => setTimeout(r, 5));
			estado.ordenes.push(si);
			estado.completa = si;
		},
	}),
}));

const { alternarPantallaCompleta, esTeclaPantallaCompleta, useVentana } = await import(
	"../ventana"
);

const tecla = (key: string, extra: Partial<KeyboardEvent> = {}) => ({
	key,
	ctrlKey: false,
	altKey: false,
	metaKey: false,
	shiftKey: false,
	...extra,
});

describe("esTeclaPantallaCompleta", () => {
	it("es F11 solo", () => {
		expect(esTeclaPantallaCompleta(tecla("F11"))).toBe(true);
	});
	it("con Ctrl, Alt, Meta o Mayús es de otra cosa", () => {
		expect(esTeclaPantallaCompleta(tecla("F11", { ctrlKey: true }))).toBe(false);
		expect(esTeclaPantallaCompleta(tecla("F11", { altKey: true }))).toBe(false);
		expect(esTeclaPantallaCompleta(tecla("F11", { metaKey: true }))).toBe(false);
		expect(esTeclaPantallaCompleta(tecla("F11", { shiftKey: true }))).toBe(false);
	});
	it("otras teclas no", () => {
		expect(esTeclaPantallaCompleta(tecla("F10"))).toBe(false);
		expect(esTeclaPantallaCompleta(tecla("f"))).toBe(false);
	});
});

describe("alternarPantallaCompleta", () => {
	beforeEach(() => {
		estado.completa = false;
		estado.ordenes = [];
		estado.falla = false;
		useVentana.setState({ completa: false });
	});

	it("entra y apunta el estado que la ventana dice", async () => {
		await alternarPantallaCompleta();
		expect(estado.ordenes).toEqual([true]);
		expect(useVentana.getState().completa).toBe(true);
	});

	it("vuelve a salir", async () => {
		await alternarPantallaCompleta();
		await alternarPantallaCompleta();
		expect(estado.ordenes).toEqual([true, false]);
		expect(useVentana.getState().completa).toBe(false);
	});

	it("dos pulsaciones seguidas se atienden una tras otra, no a la vez", async () => {
		const a = alternarPantallaCompleta();
		const b = alternarPantallaCompleta();
		await Promise.all([a, b]);
		// Sin la cola, las dos leerían «no» y pedirían entrar las dos veces.
		expect(estado.ordenes).toEqual([true, false]);
		expect(useVentana.getState().completa).toBe(false);
	});

	it("si la ventana no deja, no revienta y el estado sigue siendo el real", async () => {
		estado.falla = true;
		await expect(alternarPantallaCompleta()).resolves.toBeUndefined();
		expect(useVentana.getState().completa).toBe(false);
		// Y la cola no se queda rota para la siguiente.
		estado.falla = false;
		await alternarPantallaCompleta();
		expect(useVentana.getState().completa).toBe(true);
	});
});
