import type { AppError } from "@/bindings/AppError";
import type { DecisionPasar } from "@/bindings/DecisionPasar";
import type { DeckRevisado } from "@/bindings/DeckRevisado";
import type { EleccionPasar } from "@/bindings/EleccionPasar";
import type { EstadoEmisor } from "@/bindings/EstadoEmisor";
import type { EstadoReceptor } from "@/bindings/EstadoReceptor";
import type { MasReciente } from "@/bindings/MasReciente";
import type { ProgresoPasar } from "@/bindings/ProgresoPasar";
import type { ResolucionPasar } from "@/bindings/ResolucionPasar";
import { paraTraducir } from "@/components/language-provider";

/**
 * La lógica pura de las pantallas de pasar decks (sin React ni Rust): lo que
 * elige la persona de cada deck de la oferta y qué hace falta para poder
 * seguir. La pantalla solo pinta esto; lo decide aquí.
 */

/** Lo que la persona ha elegido de un deck de la oferta. */
export type Elegido = {
	eleccion: EleccionPasar;
	/** Solo cuenta si el deck ya existe aquí y no se ha dicho «No». */
	resolucion: ResolucionPasar | null;
};

/** Al llegar la oferta: todo «con proxies» (o el deck tal cual) y nada resuelto. */
export function eleccionesIniciales(oferta: DeckRevisado[]): Elegido[] {
	return oferta.map(() => ({ eleccion: "conProxies", resolucion: null }));
}

/** Los decks aceptados que ya existen aquí y aún no dicen qué hacer. */
export function faltaResolver(oferta: DeckRevisado[], elegidos: Elegido[]): number[] {
	return oferta
		.map((d, i) =>
			d.conflicto && elegidos[i]?.eleccion !== "no" && !elegidos[i]?.resolucion ? i : -1,
		)
		.filter((i) => i >= 0);
}

/** ¿Se acepta al menos un deck? */
export function quedaAlgo(elegidos: Elegido[]): boolean {
	return elegidos.some((e) => e.eleccion !== "no");
}

/** Se puede mandar la respuesta: algo aceptado y nada por resolver. */
export function puedeResponder(oferta: DeckRevisado[], elegidos: Elegido[]): boolean {
	return elegidos.length === oferta.length && faltaResolver(oferta, elegidos).length === 0;
}

/** Lo que se manda a Rust. */
export function aDecisiones(oferta: DeckRevisado[], elegidos: Elegido[]): DecisionPasar[] {
	return oferta.map((d, i) => {
		const e = elegidos[i] ?? { eleccion: "no" as const, resolucion: null };
		return {
			indice: i,
			eleccion: e.eleccion,
			resolucion: d.conflicto && e.eleccion !== "no" ? e.resolucion : null,
		};
	});
}

/** Los decks de aquí que se van a reemplazar (sus ids). */
export function reemplazados(oferta: DeckRevisado[], elegidos: Elegido[]): string[] {
	return oferta.flatMap((d, i) => {
		const e = elegidos[i];
		return d.conflicto && e && e.eleccion !== "no" && e.resolucion === "reemplazar"
			? [d.conflicto.deckId]
			: [];
	});
}

/** «Solo cartas originales» solo tiene sentido si el deck trae proxies. */
export function opcionesDe(d: DeckRevisado): EleccionPasar[] {
	return d.deck.proxies ? ["conProxies", "soloOriginales", "no"] : ["conProxies", "no"];
}

/** Cuál de las dos copias es más reciente, para el aviso de «ya lo tienes». */
export function quienEsMasReciente(m: MasReciente): "aqui" | "llega" | "igual" | "desconocido" {
	return m;
}

/** Lo que pesa algo, en una frase corta (`1,2 MB`, `340 KB`). */
export function pesar(bytes: number, locale = "es"): string {
	const n = (v: number, dec: number) =>
		new Intl.NumberFormat(locale, { maximumFractionDigits: dec }).format(v);
	if (bytes < 1024) return `${n(bytes, 0)} B`;
	if (bytes < 1024 * 1024) return `${n(bytes / 1024, 0)} KB`;
	if (bytes < 1024 * 1024 * 1024)
		return `${n(bytes / 1024 / 1024, bytes < 10 * 1024 * 1024 ? 1 : 0)} MB`;
	return `${n(bytes / 1024 / 1024 / 1024, 1)} GB`;
}

/** El código de 6 cifras en dos grupos de tres (`123 456`). */
export function formatoCodigo6(c: string): string {
	return c.length === 6 ? `${c.slice(0, 3)} ${c.slice(3)}` : c;
}

/** Lo que falta para que caduque el código, como `4:32` (nunca negativo). */
export function cuentaAtras(caducaMs: number, ahoraMs: number): string {
	const s = Math.max(0, Math.ceil((caducaMs - ahoraMs) / 1000));
	return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

/** Cuánto va (0–100) el deck que se mueve, sumando los que ya acabaron. */
export function porcentaje(p: ProgresoPasar | null): number {
	if (!p || p.decksTotal === 0) return 0;
	const deck = p.total > 0 ? p.hechos / p.total : 0;
	return Math.min(100, Math.round(((p.decksHechos + deck) / p.decksTotal) * 100));
}

/** Hay un intercambio en curso que cerrar la pantalla cancelaría. */
export function receptorEnCurso(e: EstadoReceptor | null): boolean {
	return !!e && ["peticion", "conectado", "oferta", "recibiendo"].includes(e.fase);
}

export function receptorTerminado(e: EstadoReceptor | null): boolean {
	return !!e && ["terminado", "cancelado", "caducado", "error"].includes(e.fase);
}

export function emisorEnCurso(e: EstadoEmisor | null): boolean {
	return !!e && !["terminado", "cancelado", "error"].includes(e.fase);
}

/** Los textos de lo que puede salir mal. Los datos de Rust solo traen el `motivo`. */
const MOTIVOS: Record<string, string> = {
	codigo_de_partida: paraTraducir(
		"Ese es el código de una partida (empieza por PB1-). Aquí hace falta el de «Recibir un deck» (empieza por PBD1-).",
	),
	codigo_de_deck: paraTraducir(
		"Ese es el código de «Recibir un deck» (empieza por PBD1-), no el de una partida.",
	),
	codigo: paraTraducir("Ese código no es válido: cópialo entero, empieza por PBD1-."),
	caducado: paraTraducir("El código ha caducado: pide otro en el equipo que recibe."),
	rechazado: paraTraducir("El otro equipo no ha aceptado el envío."),
	ocupado: paraTraducir("El otro equipo ya está recibiendo de alguien: espera un momento."),
	version: paraTraducir("Los dos equipos tienen versiones distintas de la app: actualizadlos."),
	no_espera: paraTraducir(
		"El otro equipo no está esperando un deck. Abre «Recibir» en él y usa el código nuevo.",
	),
	no_esta: paraTraducir("Ese equipo ya no está cerca: mira la lista otra vez."),
	uno_mismo: paraTraducir("Ese código es de este mismo equipo."),
	conexion: paraTraducir(
		"No se ha podido llegar al otro equipo. Comprobad que los dos tenéis conexión a internet o que estáis en la misma red.",
	),
	tiempo: paraTraducir("El otro equipo ha tardado demasiado en responder."),
	protocolo: paraTraducir("El otro equipo ha mandado algo que esta app no entiende."),
	mdns: paraTraducir("Este equipo no puede buscar otros en la red local."),
	huella: paraTraducir("El deck ha llegado dañado y no se ha guardado. Probad otra vez."),
	paquete: paraTraducir("El otro equipo ha mandado algo que esta app no entiende."),
	oferta: paraTraducir("El otro equipo ha mandado algo que esta app no entiende."),
	oferta_grande: paraTraducir("Son demasiados decks para enviar de una vez: manda menos."),
	paquete_grande: paraTraducir(
		"Un deck pesa demasiado para enviarlo (más de 500 MB). Prueba a mandarlo solo con las cartas originales.",
	),
	envio_grande: paraTraducir(
		"Todo junto pesa demasiado para enviarlo de una vez: manda menos decks.",
	),
	falta_resolucion: paraTraducir("Elige qué hacer con los decks que ya tienes."),
	sin_oferta: paraTraducir("Ya no hay nada que elegir."),
	enviando: paraTraducir("Ya hay un envío en marcha."),
	decks: paraTraducir("Elige entre 1 y 32 decks."),
};

/**
 * Un error de pasar decks en una frase: el texto de su `motivo` si lo tiene
 * aquí y, si no, el general de su código (`mensaje`, el de `useLang`).
 */
export function textoDeError(
	e: AppError | unknown,
	t: (es: string, vars?: Record<string, string | number>) => string,
	mensaje: (e: unknown) => string,
): string {
	const motivo = (e as AppError | undefined)?.params?.motivo;
	const texto = motivo ? MOTIVOS[motivo] : undefined;
	return texto ? t(texto) : mensaje(e);
}

/** ¿Es este código de «Recibir» (`PBD1-…`) o un texto pegado que lo parece? */
export function pareceCodigoDeDeck(texto: string): boolean {
	return texto.replace(/\s+/g, "").startsWith("PBD1-");
}
