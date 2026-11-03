import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { crearDetector } from "@/lib/pulsacion-larga";

describe("crearDetector", () => {
	beforeEach(() => vi.useFakeTimers());
	afterEach(() => vi.useRealTimers());

	it("salta a los 500 ms quieto, con donde empezó", () => {
		const f = vi.fn();
		const d = crearDetector(f);
		d.empezar(10, 20);
		vi.advanceTimersByTime(499);
		expect(f).not.toHaveBeenCalled();
		vi.advanceTimersByTime(1);
		expect(f).toHaveBeenCalledWith(10, 20);
		expect(d.consumir()).toBe(true);
		expect(d.consumir()).toBe(false);
	});

	it("no salta si el dedo se mueve más de 8 px", () => {
		const f = vi.fn();
		const d = crearDetector(f);
		d.empezar(0, 0);
		d.mover(9, 0);
		vi.advanceTimersByTime(1000);
		expect(f).not.toHaveBeenCalled();
		expect(d.consumir()).toBe(false);
	});

	it("aguanta un temblor de 8 px o menos", () => {
		const f = vi.fn();
		const d = crearDetector(f);
		d.empezar(0, 0);
		d.mover(5, 6);
		vi.advanceTimersByTime(500);
		expect(f).toHaveBeenCalledTimes(1);
	});

	it("no salta si se suelta antes", () => {
		const f = vi.fn();
		const d = crearDetector(f);
		d.empezar(0, 0);
		vi.advanceTimersByTime(300);
		d.cancelar();
		vi.advanceTimersByTime(1000);
		expect(f).not.toHaveBeenCalled();
	});

	it("empezar de nuevo reinicia la cuenta", () => {
		const f = vi.fn();
		const d = crearDetector(f);
		d.empezar(0, 0);
		vi.advanceTimersByTime(400);
		d.empezar(50, 50);
		vi.advanceTimersByTime(400);
		expect(f).not.toHaveBeenCalled();
		vi.advanceTimersByTime(100);
		expect(f).toHaveBeenCalledWith(50, 50);
	});
});
