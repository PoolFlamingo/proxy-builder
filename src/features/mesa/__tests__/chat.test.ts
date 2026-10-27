import { describe, expect, it } from "vitest";
import type { Evento } from "@/bindings/Evento";
import { ATAJOS } from "../atajos";
import {
	AVISOS,
	mensajeDeEvento,
	mensajesDeChat,
	SENALES,
	senalTrasElegir,
	teclaDeComunicacion,
} from "../cuentas-chat";

const ev = (seq: number, id: number, cambio: Evento["cambio"]): Evento => ({
	seq,
	tMs: seq * 1000,
	actor: { tipo: "jugador", id },
	cambio,
});
const chat = (seq: number, id: number, texto: string) =>
	ev(seq, id, { tipo: "accion", accion: { que: "chat", texto } });

describe("mensajes del chat", () => {
	const registro: Evento[] = [
		chat(1, 0, "hola"),
		ev(2, 1, { tipo: "accion", accion: { que: "aviso", tipo: "espera" } }),
		ev(3, 1, { tipo: "accion", accion: { que: "robar", n: 1 } }),
		chat(4, 2, "ruido"),
		{ ...chat(5, 0, "de la mesa"), actor: { tipo: "mesa" } },
	];
	it("solo salen el chat y los avisos de jugadores", () => {
		expect(mensajesDeChat(registro, []).map((m) => m.seq)).toEqual([1, 2, 4]);
		expect(mensajeDeEvento(registro[2] as Evento)).toBeNull();
		expect(mensajeDeEvento(registro[1] as Evento)?.aviso).toBe("espera");
	});
	it("silenciar a alguien oculta sus mensajes y avisos", () => {
		expect(mensajesDeChat(registro, [1]).map((m) => m.seq)).toEqual([1, 4]);
		expect(mensajesDeChat(registro, [0, 1, 2])).toEqual([]);
	});
});

describe("teclas de avisos y señales", () => {
	const e = (code: string, o: Partial<KeyboardEvent> = {}) => ({
		code,
		ctrlKey: false,
		metaKey: false,
		shiftKey: false,
		altKey: false,
		...o,
	});
	it("Alt+dígito y Ctrl+Mayús+dígito, por el código físico", () => {
		expect(teclaDeComunicacion(e("Digit1", { altKey: true }))).toBe("Alt+1");
		expect(teclaDeComunicacion(e("Digit3", { ctrlKey: true, shiftKey: true }))).toBe(
			"Ctrl+Mayús+3",
		);
		expect(teclaDeComunicacion(e("Digit3", { metaKey: true, shiftKey: true }))).toBe(
			"Ctrl+Mayús+3",
		);
	});
	it("otras combinaciones no son de comunicación", () => {
		expect(teclaDeComunicacion(e("Digit1"))).toBeNull();
		expect(teclaDeComunicacion(e("Digit1", { ctrlKey: true }))).toBeNull();
		expect(teclaDeComunicacion(e("KeyA", { altKey: true }))).toBeNull();
	});
	it("cada aviso y cada señal tiene su atajo en la tabla, sin repetir", () => {
		const teclas = ATAJOS.filter((a) => a.contexto === "mesa").map((a) => a.tecla);
		for (const a of AVISOS) expect(teclas).toContain(a.tecla);
		for (const s of SENALES) expect(teclas).toContain(s.tecla);
		expect(new Set(teclas).size).toBe(teclas.length);
	});
});

describe("señales", () => {
	it("la misma otra vez la quita", () => {
		expect(senalTrasElegir(null, "espera")).toBe("espera");
		expect(senalTrasElegir("espera", "espera")).toBeNull();
		expect(senalTrasElegir("espera", "pensando")).toBe("pensando");
	});
});
