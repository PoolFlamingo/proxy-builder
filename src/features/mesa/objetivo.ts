import { toast } from "sonner";
import type { IdJugador } from "@/bindings/IdJugador";
import type { VistaCarta } from "@/bindings/VistaCarta";
import { useMesa } from "@/stores/mesa-store";
import { hacer } from "./acciones";
import { ataqueAUnCompanero } from "./equipos";
import * as op from "./operaciones";
import { usePantalla } from "./pantalla";

type Traducir = (texto: string) => string;

/** Avisa si un ataque va contra un compañero de equipo y el fuego amigo está apagado. */
function avisarFuegoAmigo(
	de: VistaCarta | undefined,
	contra: IdJugador,
	t: Traducir,
	md: { flecha: "ataque" | "objetivo" },
) {
	const m = useMesa.getState();
	if (md.flecha !== "ataque" || !de) return;
	if (ataqueAUnCompanero(m.jugadores, m.variantes, de.controlador, contra)) {
		toast.warning(t("Ese jugador es de tu equipo y el fuego amigo está apagado."));
	}
}

/**
 * Con el modo de elegir objetivo en marcha (adjuntar, flecha), esta carta es la
 * elegida: se hace lo que tocaba y el modo termina. Devuelve `false` si no hay
 * modo (la carta se pulsa como siempre). Sirve para las cartas de cualquier
 * mesa, también la que se mira en pequeño en una partida local.
 */
export function elegirCarta(c: VistaCarta, t: Traducir): boolean {
	const pantalla = usePantalla.getState();
	const md = pantalla.modo;
	if (!md) return false;
	pantalla.setModo(null);
	if (md.tipo === "adjuntar" && md.carta !== c.id) {
		void op.adjuntar(md.carta, c.id);
	} else if (md.tipo === "flecha" && md.de !== c.id) {
		avisarFuegoAmigo(useMesa.getState().cartas[md.de], c.controlador, t, md);
		void hacer({
			que: "flecha",
			de: { tipo: "carta", id: md.de },
			a: { tipo: "carta", id: c.id },
			tipo: md.flecha,
		});
	}
	return true;
}

/**
 * Con una flecha en marcha, este jugador es el objetivo (pulsar su mesa).
 * Devuelve `false` si no hay una flecha que poner (adjuntar solo admite cartas).
 */
export function elegirJugador(jugador: IdJugador, t: Traducir): boolean {
	const pantalla = usePantalla.getState();
	const md = pantalla.modo;
	if (md?.tipo !== "flecha") return false;
	pantalla.setModo(null);
	avisarFuegoAmigo(useMesa.getState().cartas[md.de], jugador, t, md);
	void hacer({
		que: "flecha",
		de: { tipo: "carta", id: md.de },
		a: { tipo: "jugador", id: jugador },
		tipo: md.flecha,
	});
	return true;
}
