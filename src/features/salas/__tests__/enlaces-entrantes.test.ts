import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("sonner", () => ({ toast: vi.fn() }));

import { toast } from "sonner";
import { useMesa } from "@/stores/mesa-store";
import { useSalas } from "@/stores/salas-store";
import { aceptarEnlace } from "../enlaces-entrantes";

describe("enlaces de invitación entrantes", () => {
	beforeEach(() => {
		useSalas.getState().poner({ enlaceEntrante: null });
		useMesa.setState({ activa: false });
		vi.mocked(toast).mockClear();
	});

	it("guarda el código para que «Jugar con amigos» lo ponga", () => {
		expect(aceptarEnlace("ABCDE12345")).toBe(true);
		expect(useSalas.getState().enlaceEntrante).toBe("ABCDE12345");
		expect(toast).not.toHaveBeenCalled();
	});

	it("con una partida en marcha avisa y no guarda nada", () => {
		useMesa.setState({ activa: true });
		expect(aceptarEnlace("ABCDE12345")).toBe(false);
		expect(useSalas.getState().enlaceEntrante).toBeNull();
		expect(toast).toHaveBeenCalledOnce();
	});
});
