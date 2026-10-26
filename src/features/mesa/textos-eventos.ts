import type { Accion } from "@/bindings/Accion";
import type { Destino } from "@/bindings/Destino";
import type { Evento } from "@/bindings/Evento";
import type { IdCarta } from "@/bindings/IdCarta";
import { paraTraducir } from "@/components/language-provider";
import type { Mesa } from "@/stores/mesa-store";
import { lineaRegistro } from "./sparring/textos";

type T = (es: string, vars?: Record<string, string | number>) => string;

/**
 * **El registro en palabras**: cada evento llega como dato (`Cambio`) y aquí
 * se escribe con `t()` en el idioma de la interfaz. Rust nunca escribe
 * prosa. Lo oculto se escribe según quien mira: «roba 2 cartas», nunca
 * cuáles; una carta que no se ve es «una carta».
 */

/** Las acciones y su frase (con `{quien}` y lo demás). */
export const FRASES: Record<Accion["que"], string> = {
	mover: paraTraducir("{quien} mueve {cartas} {destino}"),
	girar: paraTraducir("{quien} gira {cartas}"),
	bocaAbajo: paraTraducir("{quien} pone boca abajo {cartas}"),
	cambiarCara: paraTraducir("{quien} transforma {carta}"),
	contador: paraTraducir("{quien} cambia los contadores {tipo} de {cartas} ({delta})"),
	fijarContador: paraTraducir("{quien} deja {carta} con {valor} contadores {tipo}"),
	multiplicarContadores: paraTraducir("{quien} multiplica los contadores de {carta}"),
	contadorClave: paraTraducir("{quien} cambia {palabra} en {cartas}"),
	pt: paraTraducir("{quien} cambia la fuerza y resistencia de {carta}"),
	masXMasY: paraTraducir("{quien} da {x}/{y} a {cartas}"),
	etiqueta: paraTraducir("{quien} cambia las etiquetas de {carta}"),
	ponerBolita: paraTraducir("{quien} pone una bolita"),
	moverBolita: paraTraducir("{quien} mueve una bolita"),
	quitarBolita: paraTraducir("{quien} quita una bolita"),
	adjuntar: paraTraducir("{quien} adjunta {carta}"),
	faseFuera: paraTraducir("{quien} cambia la fase de {cartas}"),
	noEndereza: paraTraducir("{quien} cambia si {cartas} se endereza"),
	resaltar: paraTraducir("{quien} resalta {cartas}"),
	copiaFicha: paraTraducir("{quien} crea una copia de {cartas}"),
	darControl: paraTraducir("{quien} da el control de {cartas}"),
	habilidadEnPila: paraTraducir("{quien} pone una habilidad de {carta} en la pila"),
	flecha: paraTraducir("{quien} señala un objetivo"),
	quitarFlechas: paraTraducir("{quien} quita sus flechas"),
	robar: paraTraducir("{quien} roba {n} cartas"),
	moler: paraTraducir("{quien} muele {n} cartas"),
	manifestar: paraTraducir("{quien} manifiesta {n} cartas"),
	exiliarDeArriba: paraTraducir("{quien} exilia {n} cartas de su biblioteca"),
	mirar: paraTraducir("{quien} mira {n} cartas de una biblioteca"),
	ordenar: paraTraducir("{quien} coloca las cartas que miraba"),
	revelarArriba: paraTraducir("{quien} revela {n} cartas de su biblioteca"),
	excavar: paraTraducir("{quien} revela cartas de su biblioteca"),
	resolverExcavacion: paraTraducir("{quien} decide qué hace con lo revelado"),
	buscar: paraTraducir("{quien} está buscando en su biblioteca"),
	dejarDeMirar: paraTraducir("{quien} deja de mirar su biblioteca"),
	deArribaRevelada: paraTraducir("{quien} cambia si juega con la de arriba revelada"),
	barajar: paraTraducir("{quien} baraja su biblioteca"),
	rueda: paraTraducir("{quien} hace una rueda"),
	enderezarTodo: paraTraducir("{quien} endereza todo"),
	organizar: paraTraducir("{quien} organiza su campo"),
	crearFicha: paraTraducir("{quien} crea {n} fichas"),
	mulligan: paraTraducir("{quien} hace mulligan"),
	quedarseCon: paraTraducir("{quien} se queda su mano"),
	vida: paraTraducir("{a}: {delta} vidas"),
	vidaATodos: paraTraducir("{quien}: {delta} vidas a todos"),
	drenar: paraTraducir("{quien} drena {n}"),
	extorsion: paraTraducir("{quien} extorsiona {n}"),
	veneno: paraTraducir("{a}: {delta} venenos"),
	danoComandante: paraTraducir("{a} recibe {delta} de daño de comandante"),
	contadorJugador: paraTraducir("{a}: {delta} {tipo}"),
	designar: paraTraducir("{quien} cambia una designación"),
	diaNoche: paraTraducir("{quien} cambia el día y la noche"),
	lanzamientos: paraTraducir("{quien} cambia las veces lanzado de {carta}"),
	sinCosteExtra: paraTraducir("{quien} cambia el impuesto de {carta}"),
	revelarMano: paraTraducir("{quien} cambia si su mano está revelada"),
	revelarA: paraTraducir("{quien} enseña {cartas}"),
	jugarBocaAbajo: paraTraducir("{quien} juega una carta boca abajo"),
	descartar: paraTraducir("{quien} descarta"),
	eliminar: paraTraducir("{a} queda fuera de la partida"),
	conceder: paraTraducir("{quien} concede"),
	vidaFantasma: paraTraducir("Fantasma: {delta} vidas"),
	danoComandanteFantasma: paraTraducir("El fantasma recibe {delta} de daño de comandante"),
	verFantasma: paraTraducir("{quien} cambia si se ve el fantasma"),
	senal: paraTraducir("{quien} cambia su señal"),
	pasarTurno: paraTraducir("{quien} pasa el turno"),
	turnoExtra: paraTraducir("{a} tendrá un turno extra"),
	pausar: paraTraducir("{quien} pausa o reanuda la partida"),
	saltarTurno: paraTraducir("{quien} salta el turno de {a}"),
	reiniciar: paraTraducir("{quien} empieza otra partida"),
	tirarDados: paraTraducir("{quien} tira dados"),
	moneda: paraTraducir("{quien} lanza una moneda"),
	dadoPlanar: paraTraducir("{quien} tira el dado planar"),
	planeswalk: paraTraducir("{quien} hace planeswalk"),
	ponerEnMarcha: paraTraducir("{quien} pone en marcha un plan"),
	abandonarPlan: paraTraducir("{quien} abandona {carta}"),
	resolver: paraTraducir("{quien} resuelve {carta}"),
	apilarDespues: paraTraducir("{quien} pone {carta} en la pila"),
	amontonar: paraTraducir("{quien} amontona {cartas}"),
	sacarDelMonton: paraTraducir("{quien} saca {n} del montón"),
	cambiarMonton: paraTraducir("{quien} cambia un montón ({delta})"),
	separar: paraTraducir("{quien} separa un montón"),
	agrupar: paraTraducir("{quien} agrupa {cartas}"),
	alAzarDe: paraTraducir("{quien} saca una carta al azar"),
	todoAlFondoAlAzar: paraTraducir("{quien} pone una zona entera al fondo de su biblioteca"),
	barajarTodoDentro: paraTraducir("{quien} baraja una zona entera en su biblioteca"),
	exiliarTodo: paraTraducir("{quien} exilia una zona entera"),
	intensidadSparring: paraTraducir("{quien} cambia la intensidad del sparring"),
	decidirSparring: paraTraducir("{quien} responde al fantasma"),
	chat: paraTraducir("{quien}: {texto}"),
	aviso: paraTraducir("{quien} avisa"),
	terminar: paraTraducir("{quien} termina la partida"),
	deshacer: paraTraducir("{quien} deshace"),
	rehacer: paraTraducir("{quien} rehace"),
};

/** Las que cuentan cartas, con una sola (la fuente española no tiene plural). */
const CON_UNA: Partial<Record<Accion["que"], string>> = {
	robar: paraTraducir("{quien} roba una carta"),
	moler: paraTraducir("{quien} muele una carta"),
	manifestar: paraTraducir("{quien} manifiesta una carta"),
	exiliarDeArriba: paraTraducir("{quien} exilia la carta de arriba de su biblioteca"),
	mirar: paraTraducir("{quien} mira una carta de una biblioteca"),
	revelarArriba: paraTraducir("{quien} revela la carta de arriba de su biblioteca"),
	crearFicha: paraTraducir("{quien} crea una ficha"),
};

const DESTINOS: Record<string, string> = {
	biblioteca: paraTraducir("a la biblioteca"),
	mano: paraTraducir("a la mano"),
	campo: paraTraducir("al campo"),
	cementerio: paraTraducir("al cementerio"),
	exilio: paraTraducir("al exilio"),
	mando: paraTraducir("al mando"),
	banquillo: paraTraducir("al banquillo"),
	planes: paraTraducir("a los planes"),
	planesEnMarcha: paraTraducir("a los planes en marcha"),
	pila: paraTraducir("a la pila"),
	planos: paraTraducir("a los planos"),
	planoActivo: paraTraducir("al plano activo"),
};

function nombreCarta(m: Pick<Mesa, "cartas">, id: IdCarta, t: T): string {
	return m.cartas[id]?.origen?.nombre ?? t("una carta");
}

function nombresCartas(m: Pick<Mesa, "cartas">, ids: IdCarta[], t: T): string {
	if (ids.length > 3) return t("{n} cartas", { n: ids.length });
	return ids.map((id) => nombreCarta(m, id, t)).join(", ");
}

function destino(d: Destino, t: T): string {
	return t(DESTINOS[d.zona.zona] ?? "");
}

/** La frase de un evento (vista de quien mira ahora). */
export function textoDeEvento(ev: Evento, m: Pick<Mesa, "cartas" | "jugadores">, t: T): string {
	const quien =
		ev.actor.tipo === "jugador" ? (m.jugadores[ev.actor.id]?.nombre ?? "?") : t("La mesa");
	const c = ev.cambio;
	switch (c.tipo) {
		case "dados":
			return t("{quien} tira: {tiradas}", {
				quien,
				tiradas: c.tiradas.map((x) => `d${x.caras}=${x.valor}`).join(", "),
			});
		case "monedas":
			return t("{quien} lanza monedas: {resultado}", {
				quien,
				resultado: c.lanzamientos
					.map((l) => l.map((x) => (x ? t("cara") : t("cruz"))).join("/"))
					.join(", "),
			});
		case "dadoPlanar":
			return t("{quien} tira el dado planar: {cara}", {
				quien,
				cara: c.cara === "planeswalk" ? t("Planeswalk") : c.cara === "caos" ? t("Caos") : t("Nada"),
			});
		case "restaurar":
			return c.deshecho === null
				? t("{quien} empieza otra partida", { quien })
				: t("{quien} deshace", { quien });
		case "sparring": {
			const r = c.reaccion;
			return lineaRegistro(r, r.objetivo.length > 0 ? nombresCartas(m, r.objetivo, t) : "", t);
		}
		case "accion":
			break;
	}
	const a = c.accion;
	if (a.que === "decidirSparring") {
		return a.decision === "aceptar"
			? t("{quien} resuelve la reacción del fantasma", { quien })
			: a.decision === "teniaRespuesta"
				? t("{quien} tenía respuesta para el fantasma", { quien })
				: t("{quien} dice que la reacción del fantasma no aplica", { quien });
	}
	const vars: Record<string, string | number> = { quien };
	if ("cartas" in a && Array.isArray(a.cartas)) vars.cartas = nombresCartas(m, a.cartas, t);
	if ("carta" in a && typeof a.carta === "number") vars.carta = nombreCarta(m, a.carta, t);
	if ("n" in a && typeof a.n === "number") vars.n = a.n;
	if ("delta" in a && typeof a.delta === "number")
		vars.delta = a.delta > 0 ? `+${a.delta}` : a.delta;
	if ("jugador" in a && typeof a.jugador === "number")
		vars.a = m.jugadores[a.jugador]?.nombre ?? "?";
	if ("tipo" in a && typeof a.tipo === "string") vars.tipo = a.tipo;
	if (a.que === "mover") vars.destino = destino(a.a, t);
	if (a.que === "fijarContador") vars.valor = a.valor;
	if (a.que === "contadorClave") vars.palabra = a.palabra;
	if (a.que === "masXMasY") {
		vars.x = a.x;
		vars.y = a.y;
	}
	if (a.que === "crearFicha") vars.n = a.cantidad;
	if (a.que === "chat") vars.texto = a.texto;
	if (a.que === "eliminar") vars.a = m.jugadores[a.jugador]?.nombre ?? "?";
	const una = vars.n === 1 ? CON_UNA[a.que] : undefined;
	return t(una ?? FRASES[a.que], vars);
}
