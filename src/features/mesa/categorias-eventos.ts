import type { Accion } from "@/bindings/Accion";
import type { Cambio } from "@/bindings/Cambio";
import type { Evento } from "@/bindings/Evento";

/**
 * **A qué categoría pertenece cada evento** del registro: da el icono y el
 * color de cada línea. Un `Record` sobre `Accion["que"]` y `Cambio["tipo"]`
 * obliga a clasificar cada variante nueva (el compilador avisa) y un test
 * comprueba que también tiene frase.
 */
export type CategoriaEvento =
	| "mover"
	| "girar"
	| "contadores"
	| "vida"
	| "mando"
	| "biblioteca"
	| "turno"
	| "azar"
	| "chat"
	| "fin"
	| "sparring"
	| "deshacer"
	| "otro";

export const CATEGORIAS_DE_ACCION: Record<Accion["que"], CategoriaEvento> = {
	mover: "mover",
	girar: "girar",
	bocaAbajo: "girar",
	cambiarCara: "girar",
	contador: "contadores",
	fijarContador: "contadores",
	multiplicarContadores: "contadores",
	contadorClave: "contadores",
	pt: "contadores",
	masXMasY: "contadores",
	etiqueta: "contadores",
	ponerBolita: "otro",
	moverBolita: "otro",
	quitarBolita: "otro",
	adjuntar: "mover",
	faseFuera: "girar",
	noEndereza: "girar",
	resaltar: "otro",
	copiaFicha: "mover",
	darControl: "mover",
	tomarControl: "mover",
	porJugador: "otro",
	mirarMano: "biblioteca",
	habilidadEnPila: "mover",
	flecha: "otro",
	quitarFlechas: "otro",
	robar: "biblioteca",
	moler: "biblioteca",
	manifestar: "biblioteca",
	exiliarDeArriba: "biblioteca",
	mirar: "biblioteca",
	ordenar: "biblioteca",
	revelarArriba: "biblioteca",
	excavar: "biblioteca",
	resolverExcavacion: "biblioteca",
	buscar: "biblioteca",
	dejarDeMirar: "biblioteca",
	deArribaRevelada: "biblioteca",
	barajar: "biblioteca",
	rueda: "biblioteca",
	enderezarTodo: "girar",
	organizar: "otro",
	crearFicha: "mover",
	mulligan: "biblioteca",
	quedarseCon: "biblioteca",
	vida: "vida",
	vidaATodos: "vida",
	drenar: "vida",
	extorsion: "vida",
	veneno: "vida",
	danoComandante: "mando",
	contadorJugador: "vida",
	designar: "otro",
	diaNoche: "otro",
	lanzamientos: "contadores",
	sinCosteExtra: "contadores",
	revelarMano: "biblioteca",
	revelarA: "biblioteca",
	jugarBocaAbajo: "mover",
	descartar: "mover",
	eliminar: "fin",
	conceder: "fin",
	vidaFantasma: "sparring",
	danoComandanteFantasma: "sparring",
	verFantasma: "sparring",
	senal: "chat",
	pasarTurno: "turno",
	turnoExtra: "turno",
	pausar: "turno",
	saltarTurno: "turno",
	reiniciar: "fin",
	tirarDados: "azar",
	moneda: "azar",
	dadoPlanar: "azar",
	planeswalk: "mover",
	ponerEnMarcha: "mover",
	abandonarPlan: "mover",
	resolver: "mover",
	apilarDespues: "mover",
	amontonar: "mover",
	sacarDelMonton: "mover",
	cambiarMonton: "mover",
	separar: "mover",
	agrupar: "mover",
	alAzarDe: "azar",
	todoAlFondoAlAzar: "biblioteca",
	barajarTodoDentro: "biblioteca",
	exiliarTodo: "mover",
	intensidadSparring: "sparring",
	decidirSparring: "sparring",
	chat: "chat",
	aviso: "chat",
	terminar: "fin",
	deshacer: "deshacer",
	rehacer: "deshacer",
};

export const CATEGORIAS_DE_CAMBIO: Record<Cambio["tipo"], CategoriaEvento | null> = {
	/** La acción: la categoría la da cada `Accion`. */
	accion: null,
	dados: "azar",
	monedas: "azar",
	dadoPlanar: "azar",
	restaurar: "deshacer",
	sparring: "sparring",
};

export function categoriaDeEvento(ev: Evento): CategoriaEvento {
	const c = ev.cambio;
	if (c.tipo === "accion") {
		// Lo hecho por otro jugador cuenta como lo que se hizo.
		const a = c.accion.que === "porJugador" ? c.accion.accion : c.accion;
		return CATEGORIAS_DE_ACCION[a.que];
	}
	if (c.tipo === "restaurar" && c.deshecho === null) return "fin";
	return CATEGORIAS_DE_CAMBIO[c.tipo] ?? "otro";
}
