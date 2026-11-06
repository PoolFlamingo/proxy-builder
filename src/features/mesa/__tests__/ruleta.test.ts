// @vitest-environment happy-dom
import { describe, expect, it } from "vitest";
import type { VistaMesa } from "@/bindings/VistaMesa";
import { useUi } from "@/stores/ui-store";
import {
	anguloFrenando,
	frenado,
	partidaSinEmpezar,
	recortarNombre,
	sectorBajoLaAguja,
	trazoSector,
	VELOCIDAD,
} from "../ruleta/cuentas";
import { anguloDe, cerrar, girar, parar, tocaRuleta, useRuleta } from "../ruleta/estado";

describe("la ruleta de quién empieza", () => {
	it("acaba siempre en quien ha decidido el motor, venga de donde venga", () => {
		for (const n of [2, 3, 4, 7, 10]) {
			for (let ganador = 0; ganador < n; ganador++) {
				for (const angulo of [0, 17, 359.5, 1234.5]) {
					for (const azar of [0, 0.5, 1]) {
						const f = frenado(angulo, ganador, n, azar);
						expect(sectorBajoLaAguja(f.objetivo, n)).toBe(ganador);
					}
				}
			}
		}
	});

	it("da al menos una vuelta más y no se pasa de dos", () => {
		for (const angulo of [0, 90, 720.25]) {
			const f = frenado(angulo, 1, 4, 0.5);
			expect(f.objetivo - angulo).toBeGreaterThanOrEqual(360);
			expect(f.objetivo - angulo).toBeLessThan(720);
		}
	});

	it("frena sin tirón: sigue a la velocidad a la que venía y no se pasa nunca", () => {
		const f = frenado(100, 0, 4, 0.5);
		const paso = 1;
		const velocidad = ((anguloFrenando(f, paso) - f.desde) / paso) * 1000;
		expect(Math.abs(velocidad - VELOCIDAD) / VELOCIDAD).toBeLessThan(0.01);
		let antes = f.desde;
		for (let ms = 0; ms <= f.ms + 50; ms += 50) {
			const a = anguloFrenando(f, ms);
			expect(a).toBeGreaterThanOrEqual(antes);
			expect(a).toBeLessThanOrEqual(f.objetivo);
			antes = a;
		}
		expect(anguloFrenando(f, f.ms)).toBe(f.objetivo);
	});

	it("tarda lo que se ve bien, ni un parpadeo ni una espera larga", () => {
		for (let ganador = 0; ganador < 10; ganador++) {
			const { ms } = frenado(33, ganador, 10, 0.2);
			expect(ms).toBeGreaterThan(1500);
			expect(ms).toBeLessThan(4000);
		}
	});

	it("el trazado de un sector empieza en el centro y se cierra", () => {
		expect(trazoSector(0, 2, 100)).toBe("M0 0 L0.000 -100.000 A100 100 0 0 1 0.000 100.000 Z");
	});

	it("recorta los nombres sin partir un emoji y teniendo en cuenta las letras anchas", () => {
		expect(recortarNombre("Ana", 6)).toBe("Ana");
		const largo = recortarNombre("Bartolomeu de Gusmão", 6);
		expect(largo.endsWith("…")).toBe(true);
		expect(largo.length).toBeLessThan(13);
		// Las de japonés ocupan el doble: caben menos.
		expect(recortarNombre("ああああああああ", 4).length).toBeLessThan(6);
		const emoji = recortarNombre("😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀", 3);
		expect(emoji.endsWith("…")).toBe(true);
		expect(Array.from(emoji).every((c) => c === "😀" || c === "…")).toBe(true);
	});

	const vista = (n: number, numero: number, decididos: number[] = []) =>
		({
			jugadores: Array.from({ length: n }),
			general: { turno: { numero }, mulligan: numero === 0 ? { decididos } : null },
		}) as unknown as Pick<VistaMesa, "jugadores" | "general">;

	it("solo se sortea en una partida de varios sin empezar", () => {
		expect(partidaSinEmpezar(vista(2, 0))).toBe(true);
		expect(partidaSinEmpezar(vista(1, 0))).toBe(false);
		// Retomar o volver: ya se decidió quién empezó.
		expect(partidaSinEmpezar(vista(2, 3))).toBe(false);
		expect(partidaSinEmpezar(vista(2, 0, [1]))).toBe(false);
	});
});

describe("el estado de la ruleta", () => {
	const sectores = [
		{ id: 0, nombre: "Ana", color: "#f00" },
		{ id: 1, nombre: "Bea", color: "#00f" },
	];

	it("con menos de dos jugadores no hay nada que sortear", () => {
		girar(sectores.slice(0, 1));
		expect(useRuleta.getState().giro).toBeNull();
	});

	it("apagada en Ajustes no sale, ni al preparar la mesa ni con la vista", () => {
		const v = {
			jugadores: sectores,
			general: { turno: { numero: 0 }, mulligan: { decididos: [] } },
		} as unknown as Pick<VistaMesa, "jugadores" | "general">;
		expect(tocaRuleta(v)).toBe(true);
		useUi.getState().setMesa({ ruletaEmpieza: false });
		expect(tocaRuleta(v)).toBe(false);
		girar(sectores);
		expect(useRuleta.getState().giro).toBeNull();
		useUi.getState().setMesa({ ruletaEmpieza: true });
	});

	it("girar y parar acaban quitándose y no se queda nada esperando", async () => {
		girar(sectores);
		const g = useRuleta.getState().giro;
		expect(g?.parada).toBeNull();
		// La vista trae los nombres de verdad: mandan esos.
		const hecho = parar(1, [sectores[0], { ...sectores[1], nombre: "Beatriz" }], true);
		const frenando = useRuleta.getState().giro;
		expect(frenando?.parada?.ganador).toBe(1);
		expect(frenando?.sectores[1].nombre).toBe("Beatriz");
		// Cerrar antes de tiempo (un fallo, salir) no deja la mesa sin abrir.
		cerrar();
		await hecho;
		expect(useRuleta.getState().giro).toBeNull();
	});

	it("el ángulo no salta al empezar a frenar", () => {
		girar(sectores);
		const giro = useRuleta.getState().giro;
		if (!giro) throw new Error("no gira");
		const ahora = giro.inicio + 1000;
		expect(anguloDe(giro, ahora)).toBeCloseTo(VELOCIDAD);
		const f = frenado(anguloDe(giro, ahora), 0, 2, 0.5);
		const conParada = { ...giro, parada: { ganador: 0, mio: false, empieza: ahora, frenado: f } };
		expect(anguloDe(conParada, ahora)).toBeCloseTo(VELOCIDAD);
		cerrar();
	});
});
