import type { FilaPartida } from "@/bindings/FilaPartida";
import type { Formato } from "@/bindings/Formato";

/**
 * **Cuentas del historial general** (J1): filtrar las partidas que ya están
 * cargadas. Rust pagina y ordena (`partidas_todas`); aquí solo lo que decide
 * qué filas se enseñan con lo que el usuario ha escrito o elegido.
 */

export type FiltroResultado = "todas" | "ganadas" | "perdidas";

export type FiltroHistorial = {
	/** Se busca en nombres de jugador, decks, listas y la nota. */
	texto: string;
	resultado: FiltroResultado;
	formato: Formato | "todos";
};

export const FILTRO_VACIO: FiltroHistorial = { texto: "", resultado: "todas", formato: "todos" };

/** Cuántos asientos se enseñan en cada fila antes del «y N más». */
export const MAX_ASIENTOS_FILA = 4;

/** Sin tildes ni mayúsculas, para que «Cien» encuentre «cien» y «Ent» a «Ént». */
function plano(s: string): string {
	return s
		.normalize("NFD")
		.replace(/\p{Diacritic}/gu, "")
		.toLowerCase();
}

/** Todo lo que se puede buscar en una partida, en una sola cadena. */
export function textoBuscable(f: FilaPartida): string {
	return plano(
		[...f.jugadores.flatMap((j) => [j.nombre, j.deck, j.lista]), f.nota]
			.filter((x) => x.length > 0)
			.join("\n"),
	);
}

/** Cada palabra escrita tiene que aparecer (en cualquier orden). */
export function coincideTexto(f: FilaPartida, texto: string): boolean {
	const palabras = plano(texto).split(/\s+/).filter(Boolean);
	if (palabras.length === 0) return true;
	const t = textoBuscable(f);
	return palabras.every((p) => t.includes(p));
}

/**
 * Las filas que cumplen el filtro. «Perdidas» son las de un deck tuyo que no
 * ganó; una partida sin deck tuyo (puesto 0: un invitado en red) no cuenta
 * ni como ganada ni como perdida.
 */
export function filtrarPartidas(filas: readonly FilaPartida[], f: FiltroHistorial): FilaPartida[] {
	return filas.filter((p) => {
		if (f.formato !== "todos" && p.formato !== f.formato) return false;
		if (f.resultado === "ganadas" && !p.gano) return false;
		if (f.resultado === "perdidas" && (p.gano || p.puesto === 0)) return false;
		return coincideTexto(p, f.texto);
	});
}

/** Cuántas de estas partidas ganó alguno de tus decks. */
export function victoriasDe(filas: readonly FilaPartida[]): number {
	return filas.filter((p) => p.gano).length;
}

/** ¿Hay algún filtro puesto? */
export function hayFiltro(f: FiltroHistorial): boolean {
	return f.texto.trim() !== "" || f.resultado !== "todas" || f.formato !== "todos";
}
