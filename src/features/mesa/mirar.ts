import { useEffect } from "react";
import { toast } from "sonner";
import type { IdJugador } from "@/bindings/IdJugador";
import { traduccionSuelta } from "@/components/language-provider";
import { mensajeDeError } from "@/lib/errores";
import { call } from "@/lib/ipc";
import { useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { useUi } from "@/stores/ui-store";
import { useDisposicion } from "./selector-disposicion";

/**
 * Una partida local de varios: más de un asiento y sin red. Aquí quien mira
 * es también quien juega, así que pasar a otro tablero es pasar a ser ese
 * jugador (con su mano a la vista).
 */
export function esLocalDeVarios(): boolean {
	return useRed.getState().papel === null && useMesa.getState().asientos.length > 1;
}

/**
 * Pasa a mirar el tablero de `id`. En una partida local de varios pide a Rust
 * que el que mira sea ese jugador (la vista entera llega por el canal); en
 * red solo cambia el tablero que se ve.
 */
export async function mirarJugador(id: IdJugador): Promise<void> {
	if (!esLocalDeVarios()) {
		useMesa.getState().mirar(id);
		return;
	}
	try {
		await call("mesa_mirar_como", { jugador: id });
		useMesa.setState({ mirando: null, seleccion: [] });
	} catch (e) {
		toast.error(mensajeDeError(e, traduccionSuelta));
	}
}

/** Con «Seguir al jugador del turno», cada turno pasa a mirar al que juega. */
export function useSeguirTurno() {
	const seguir = useUi((s) => s.mesa.seguirTurnoLocal);
	const activo = useMesa((s) => s.general?.turno.activo ?? null);
	const espectador = useRed((s) => s.papel === "espectador");
	const { config } = useDisposicion();
	// Quien solo mira, con un tablero grande y los demás en miniatura, ve el del turno.
	const enFoco = espectador && config.modo === "foco";
	useEffect(() => {
		if (enFoco && activo !== null) useMesa.getState().mirar(activo);
	}, [enFoco, activo]);
	useEffect(() => {
		if (!seguir || activo === null || !esLocalDeVarios()) return;
		if (useMesa.getState().yo !== activo) void mirarJugador(activo);
	}, [seguir, activo]);
}
