import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import type { IdJugador } from "@/bindings/IdJugador";
import type { OriginalMesa } from "@/bindings/OriginalMesa";
import { leerIdioma } from "@/components/language-provider";
import { idioma as datosDeIdioma } from "@/lib/idiomas";
import { call } from "@/lib/ipc";
import { useMesa } from "@/stores/mesa-store";
import { useOriginales } from "@/stores/originales-store";
import { pedirVista } from "./conexion";

/**
 * **«Originales en mi idioma»** (E9) desde la pantalla. Por cada jugador se
 * puede pedir ver sus cartas de Scryfall con la impresión en el idioma de la
 * interfaz (o la inglesa) en vez de sus proxies. Rust busca cada impresión y
 * baja sus imágenes (`mesa_originales`, perezoso y a su ritmo) y avisa de
 * cada una por `mesa://originales`; aquí se cambian las caras de las cartas
 * de ese jugador en el store. Es solo de esta pantalla.
 */

/** El idioma de cartas que Rust usará: los idiomas sin cartas impresas piden inglés. */
export function idiomaDeCartas(): string {
	const l = leerIdioma();
	return datosDeIdioma(l).impresas ? l : "en";
}

let escucha = false;
/** Lo que ya se pidió a Rust en este idioma (para no repetirlo en cada parche). */
const pedidas = new Set<string>();

function escuchar() {
	if (escucha) return;
	escucha = true;
	void listen<OriginalMesa>("mesa://originales", (e) => {
		if (e.payload.idioma !== idiomaDeCartas()) return;
		useOriginales.getState().recibir(e.payload);
		useMesa.getState().reaplicarOriginales();
	});
}

/** Pide a Rust las que faltan de los jugadores en este modo: primero el campo, luego la mano y el resto. */
export function pedirOriginales() {
	const o = useOriginales.getState();
	const idioma = idiomaDeCartas();
	if (o.idioma !== idioma) {
		o.olvidarCaras(idioma);
		pedidas.clear();
	}
	const orden = (z: string) => (z === "campo" ? 0 : z === "mano" ? 1 : 2);
	const cartas = Object.values(useMesa.getState().cartas)
		.filter((c) => o.modos[c.dueno] && c.origen?.scryfallId)
		.sort(
			(a, b) =>
				orden(a.zona.tipo === "de" ? a.zona.zona : "") -
				orden(b.zona.tipo === "de" ? b.zona.zona : ""),
		);
	const ids: string[] = [];
	for (const c of cartas) {
		const id = c.origen?.scryfallId;
		if (id && !pedidas.has(id) && !ids.includes(id)) ids.push(id);
	}
	if (ids.length === 0) return;
	for (const id of ids) pedidas.add(id);
	void call("mesa_originales", { ids, idioma }).catch(() => {
		for (const id of ids) pedidas.delete(id);
	});
}

/** Cambia el modo de un jugador: sus cartas pasan a los originales o vuelven a como las tiene. */
export function alternarOriginales(jugador: IdJugador) {
	escuchar();
	const o = useOriginales.getState();
	const encender = !o.modos[jugador];
	o.activar(jugador, encender);
	if (encender) {
		pedirOriginales();
		useMesa.getState().reaplicarOriginales();
	} else {
		// Las caras de antes las tiene Rust: se pide la vista entera.
		void pedirVista();
	}
}

/** Mientras la mesa está abierta, lo que llega de nuevo de esos jugadores también se pide. */
export function useOriginalesAlDia() {
	const version = useMesa((s) => s.version);
	const hay = useOriginales((s) => Object.keys(s.modos).length > 0);
	useEffect(() => {
		if (!hay) return;
		const i = window.setTimeout(pedirOriginales, 400);
		return () => window.clearTimeout(i);
	}, [hay, version]);
}
