import type { Accion } from "@/bindings/Accion";
import type { IdCarta } from "@/bindings/IdCarta";
import type { IdJugador } from "@/bindings/IdJugador";
import type { ModoExilio } from "@/bindings/ModoExilio";
import type { Zona } from "@/bindings/Zona";
import { traduccionSuelta as t } from "@/components/language-provider";
import { useMesa } from "@/stores/mesa-store";
import { useUi } from "@/stores/ui-store";
import { destino, destinoComun, hacer, mover, objetivo, sitio } from "./acciones";
import { avisarBono } from "./aviso-bono";
import { pedirNumero, usePantalla } from "./pantalla";

/**
 * **Lo que se hace con las cartas**, en un solo sitio: los menús, los atajos
 * de teclado y el arrastre llaman a estas funciones. Cada una actúa sobre la
 * selección si la carta está en ella (`objetivo`).
 */

const carta = (id: IdCarta) => useMesa.getState().cartas[id];
const yo = () => useMesa.getState().yo ?? 0;

export function girar(id: IdCarta) {
	const ids = objetivo(id);
	const girada = !carta(id)?.girada;
	return hacer({ que: "girar", cartas: ids, girada });
}

export function bocaAbajo(id: IdCarta) {
	const c = carta(id);
	if (!c) return;
	if (c.zona.tipo === "de" && c.zona.zona === "mano") {
		return hacer({ que: "jugarBocaAbajo", carta: id, pos: null });
	}
	return hacer({ que: "bocaAbajo", cartas: objetivo(id), si: !c.bocaAbajo });
}

export function transformar(id: IdCarta) {
	return hacer({ que: "cambiarCara", carta: id });
}

export function masUno(id: IdCarta, delta: number) {
	return hacer({ que: "contador", cartas: objetivo(id), tipo: "+1/+1", delta });
}

export function aZona(id: IdCarta, zona: Zona, s = sitio.arriba) {
	return mover(objetivo(id), destino(zona, s));
}

export function aLaPila(id: IdCarta) {
	return mover(objetivo(id), destinoComun("pila"));
}

export function exiliarBocaAbajo(id: IdCarta) {
	const ids = objetivo(id);
	void mover(ids, destino("exilio")).then((ok) => {
		if (ok) void hacer({ que: "bocaAbajo", cartas: ids, si: true });
	});
}

export function barajarDentro(id: IdCarta) {
	return mover(objetivo(id), destino("biblioteca", sitio.barajar));
}

export function aLaPosicion(id: IdCarta) {
	pedirNumero(t("¿En qué posición desde arriba?"), 3, (n) =>
		mover(objetivo(id), destino("biblioteca", sitio.xDesdeArriba(n))),
	);
}

export function resaltar(id: IdCarta, color: number | null) {
	return hacer({ que: "resaltar", cartas: objetivo(id), color });
}

export function alternarResaltado(id: IdCarta) {
	return resaltar(id, carta(id)?.resaltada == null ? 0 : null);
}

export function copiaFicha(id: IdCarta) {
	return hacer({ que: "copiaFicha", cartas: objetivo(id) });
}

export function faseFuera(id: IdCarta) {
	return hacer({ que: "faseFuera", cartas: objetivo(id), si: !carta(id)?.faseFuera });
}

export function noEndereza(id: IdCarta) {
	return hacer({ que: "noEndereza", cartas: objetivo(id), si: !carta(id)?.noEndereza });
}

export function soltarAdjunto(id: IdCarta) {
	return hacer({ que: "adjuntar", carta: id, a: null });
}

export function adjuntarA(id: IdCarta) {
	usePantalla.getState().setModo({ tipo: "adjuntar", carta: id });
}

/** Adjunta `id` a `a` y, si toca, ofrece su bono (`ofrecerBono`). */
export async function adjuntar(id: IdCarta, a: IdCarta) {
	if (await hacer({ que: "adjuntar", carta: id, a })) ofrecerBono(id, a);
}

const conSigno = (n: number) => (n >= 0 ? `+${n}` : `${n}`);

/**
 * Tras adjuntar un aura o un equipo con un «+N/+N» fijo («Equipped creature
 * gets +2/+2»), ofrece sumarlo a la criatura en la fila «otros» (Ajustes >
 * Mesa). El motor no lo suma solo: muchas lo condicionan y quien juega decide.
 */
export function ofrecerBono(adjunta: IdCarta, a: IdCarta) {
	if (!useUi.getState().mesa.ofrecerBono) return;
	const bono = carta(adjunta)?.origen?.info.bonoFijo;
	const criatura = carta(a)?.origen;
	if (!bono || !criatura?.info.esCriatura) return;
	const [x, y] = bono;
	avisarBono({
		equipo: carta(adjunta)?.origen?.nombre ?? "",
		criatura: criatura.nombre,
		bono: `${conSigno(x)}/${conSigno(y)}`,
		alSumar: () => void hacer({ que: "masXMasY", cartas: [a], x, y, fila: "otros" }),
	});
}

/** Da el control de estas cartas (con lo que llevan encima) a otro jugador. */
export function darControl(ids: IdCarta[], a: IdJugador) {
	return hacer({ que: "darControl", cartas: ids, a });
}

/** Toma el control de lo que controla otro, para siempre o hasta el final del turno. */
export function tomarControl(ids: IdCarta[], hastaFinDeTurno = false) {
	return hacer({ que: "tomarControl", cartas: ids, hastaFinDeTurno });
}

export function flecha(id: IdCarta, tipo: "ataque" | "objetivo" = "ataque") {
	usePantalla.getState().setModo({ tipo: "flecha", de: id, flecha: tipo });
}

export function inspeccionar(id: IdCarta) {
	useMesa.getState().inspeccionar(id);
}

export function habilidadEnPila(id: IdCarta) {
	usePantalla.getState().abrir({
		tipo: "texto",
		titulo: t("Habilidad a la pila"),
		max: 200,
		hacer: (texto) => hacer({ que: "habilidadEnPila", carta: id, texto }),
	});
}

export function sacarDelMonton(id: IdCarta, n = 1) {
	return hacer({ que: "sacarDelMonton", monton: id, n });
}

export function amontonar(id: IdCarta) {
	const ids = objetivo(id);
	if (ids.length < 2) return;
	// La pulsada, la base.
	return hacer({ que: "amontonar", cartas: [id, ...ids.filter((x) => x !== id)] });
}

// ── Biblioteca ──

/**
 * Lo hace quien juega en sus zonas o, si son de otro jugador, **por él**
 * (`porJugador`: que robe, muela, baraje, exilie su cementerio…), como si lo
 * hiciera él. En una mesa cada uno puede hacer lo que quiera.
 */
export function hacerPor(jugador: IdJugador | null | undefined, accion: Accion) {
	return jugador == null || jugador === yo()
		? hacer(accion)
		: hacer({ que: "porJugador", jugador, accion });
}

export function robar(n = 1, de?: IdJugador) {
	return hacerPor(de, { que: "robar", n });
}

export function moler(n = 1, de?: IdJugador) {
	return hacerPor(de, { que: "moler", n });
}

export function exiliarDeArriba(modo: ModoExilio, n = 1, de?: IdJugador) {
	return hacerPor(de, { que: "exiliarDeArriba", n, modo });
}

/** Mira la biblioteca de `de` (de fábrica, la propia; la de otro, Gonti, Lantern…). */
export function mirar(
	modo: "mirar" | "escrutar" | "vigilar",
	desde: "arriba" | "abajo" = "arriba",
	de: IdJugador = yo(),
) {
	const titulo =
		modo === "escrutar"
			? t("Escrutar cuántas")
			: modo === "vigilar"
				? t("Vigilar cuántas")
				: desde === "arriba"
					? t("Mirar cuántas de arriba")
					: t("Mirar cuántas de abajo");
	pedirNumero(titulo, 1, (n) => {
		void hacer({ que: "mirar", de, n, desde }).then((ok) => {
			if (ok) usePantalla.getState().abrir({ tipo: "mirar", de, n, desde, modo });
		});
	});
}

/** Busca en la biblioteca de `de` (la de otro: Bribery, Praetor's Grasp…). */
export function buscar(filtro: "todo" | "tierra" = "todo", de: IdJugador = yo()) {
	void hacer({ que: "buscar", de }).then((ok) => {
		if (ok) usePantalla.getState().abrir({ tipo: "buscar", de, filtro });
	});
}

export function barajar(de?: IdJugador) {
	return hacerPor(de, { que: "barajar" });
}

export function enderezarTodo(de?: IdJugador) {
	return hacerPor(de, { que: "enderezarTodo" });
}

/** Mira (o deja de mirar) la mano de otro jugador. */
export function mirarMano(de: IdJugador, si = true) {
	return hacer({ que: "mirarMano", de, si });
}

export function pasarTurno() {
	return hacer({ que: "pasarTurno" });
}

export function deshacer() {
	return hacer({ que: "deshacer", hasta: null });
}

export function rehacer() {
	return hacer({ que: "rehacer" });
}
