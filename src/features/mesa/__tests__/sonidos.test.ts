import { describe, expect, it } from "vitest";
import type { Accion } from "@/bindings/Accion";
import type { Evento } from "@/bindings/Evento";
import { GRUPO_DE_SONIDO, sonidoDeEvento } from "../sonidos";

const accion = (a: Accion, actor = 0): Evento => ({
	seq: 1,
	tMs: 0,
	actor: { tipo: "jugador", id: actor },
	cambio: { tipo: "accion", accion: a },
});
const mover = (zona: "campo" | "mano"): Accion => ({
	que: "mover",
	cartas: [1],
	a: { zona: { tipo: "de", zona }, jugador: null, sitio: "arriba" } as never,
	como: null,
});

describe("sonido de cada evento", () => {
	it("cartas: robar, barajar, girar y jugar", () => {
		expect(sonidoDeEvento(accion({ que: "robar", n: 1 }), 0, 0)).toBe("robar");
		expect(sonidoDeEvento(accion({ que: "barajar" }), 0, 0)).toBe("barajar");
		expect(sonidoDeEvento(accion({ que: "girar", cartas: [1] } as never), 0, 0)).toBe("girar");
		expect(sonidoDeEvento(accion(mover("campo")), 0, 0)).toBe("jugar");
		expect(sonidoDeEvento(accion(mover("mano")), 0, 0)).toBe("mover");
	});
	it("pasar el turno suena distinto si llega a mí", () => {
		expect(sonidoDeEvento(accion({ que: "pasarTurno" }), 0, 0)).toBe("tuTurno");
		expect(sonidoDeEvento(accion({ que: "pasarTurno" }), 0, 1)).toBe("turno");
		expect(sonidoDeEvento(accion({ que: "pasarTurno" }), null, 1)).toBe("turno");
	});
	it("mis mensajes no me suenan, los de otros sí", () => {
		const chat: Accion = { que: "chat", texto: "hola" };
		expect(sonidoDeEvento(accion(chat, 0), 0, 0)).toBeNull();
		expect(sonidoDeEvento(accion(chat, 1), 0, 0)).toBe("chat");
		expect(sonidoDeEvento(accion({ que: "aviso", tipo: "ok" }, 1), 0, 0)).toBe("chat");
	});
	it("lo demás no suena y otra partida baraja", () => {
		expect(sonidoDeEvento(accion({ que: "vida", jugador: 0, delta: 1 }), 0, 0)).toBeNull();
		const otra: Evento = {
			...accion({ que: "robar", n: 1 }),
			cambio: { tipo: "restaurar", deshecho: null },
		};
		expect(sonidoDeEvento(otra, 0, 0)).toBe("barajar");
		const deshacer: Evento = { ...otra, cambio: { tipo: "restaurar", deshecho: 3 } };
		expect(sonidoDeEvento(deshacer, 0, 0)).toBeNull();
	});
	it("cada sonido pertenece a un interruptor", () => {
		for (const g of Object.values(GRUPO_DE_SONIDO)) {
			expect(["cartas", "turno", "chat"]).toContain(g);
		}
	});
});
