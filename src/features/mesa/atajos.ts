import type { IdCarta } from "@/bindings/IdCarta";
import { paraTraducir } from "@/components/language-provider";
import { tableroDe, useMesa } from "@/stores/mesa-store";
import { hacer, objetivo, sitio } from "./acciones";
import { AVISOS, SENALES, senalTrasElegir } from "./cuentas-chat";
import { alternarRespuesta } from "./mirar";
import * as op from "./operaciones";
import { usePantalla } from "./pantalla";

/**
 * **Los atajos de teclado de la mesa**, en una sola tabla (la de Playgroup,
 * `inventario-playgroup.md` §17). Cada uno vale en un contexto: con el ratón
 * sobre una carta (o con cartas elegidas), sobre la biblioteca o en toda la
 * mesa. Los menús enseñan la tecla a la derecha de cada opción; `?` abre la
 * lista entera. El test comprueba que no se repite ninguna tecla en el mismo
 * contexto.
 */

export type Contexto = "carta" | "biblioteca" | "mesa";

export type Atajo = {
	contexto: Contexto;
	/** Como se escribe en `KeyboardEvent.key` (con `Mayús+` o `Ctrl+` delante). */
	tecla: string;
	nombre: string;
	hacer: (carta: IdCarta | null) => void;
};

const conCarta = (f: (id: IdCarta) => unknown) => (id: IdCarta | null) => {
	if (id !== null) void f(id);
};

/** De quién es la biblioteca bajo el ratón: la del tablero grande. */
const duenoBiblioteca = () => tableroDe(useMesa.getState());

export const ATAJOS: Atajo[] = [
	// ── Carta (bajo el ratón o elegida) ──
	{
		contexto: "carta",
		tecla: "t",
		nombre: paraTraducir("Girar / enderezar"),
		hacer: conCarta(op.girar),
	},
	{
		contexto: "carta",
		tecla: "r",
		nombre: paraTraducir("Transformar"),
		hacer: conCarta(op.transformar),
	},
	{
		contexto: "carta",
		tecla: "f",
		nombre: paraTraducir("Boca abajo"),
		hacer: conCarta(op.bocaAbajo),
	},
	{
		contexto: "carta",
		tecla: "c",
		nombre: paraTraducir("Añadir contador"),
		hacer: conCarta((id) =>
			usePantalla.getState().abrir({ tipo: "contador", cartas: objetivo(id) }),
		),
	},
	{
		contexto: "carta",
		tecla: "+",
		nombre: paraTraducir("Añadir +1/+1"),
		hacer: conCarta((id) => op.masUno(id, 1)),
	},
	{
		contexto: "carta",
		tecla: "=",
		nombre: paraTraducir("Añadir +1/+1"),
		hacer: conCarta((id) => op.masUno(id, 1)),
	},
	{
		contexto: "carta",
		tecla: "-",
		nombre: paraTraducir("Quitar +1/+1"),
		hacer: conCarta((id) => op.masUno(id, -1)),
	},
	{
		contexto: "carta",
		tecla: "m",
		nombre: paraTraducir("Mostrar a todos / dejar de mostrar"),
		hacer: conCarta(op.alternarMostrada),
	},
	{
		contexto: "carta",
		tecla: "x",
		nombre: paraTraducir("Fuerza y resistencia"),
		hacer: conCarta((id) => usePantalla.getState().abrir({ tipo: "pt", carta: id })),
	},
	{
		contexto: "carta",
		tecla: "g",
		nombre: paraTraducir("Al cementerio"),
		hacer: conCarta((id) => op.aZona(id, "cementerio")),
	},
	{
		contexto: "carta",
		tecla: "h",
		nombre: paraTraducir("A la mano"),
		hacer: conCarta((id) => op.aZona(id, "mano")),
	},
	{
		contexto: "carta",
		tecla: "e",
		nombre: paraTraducir("Al exilio"),
		hacer: conCarta((id) => op.aZona(id, "exilio")),
	},
	{
		contexto: "carta",
		tecla: "Mayús+E",
		nombre: paraTraducir("Al exilio boca abajo"),
		hacer: conCarta(op.exiliarBocaAbajo),
	},
	{
		contexto: "carta",
		tecla: "s",
		nombre: paraTraducir("Barajar en la biblioteca"),
		hacer: conCarta(op.barajarDentro),
	},
	{
		contexto: "carta",
		tecla: "b",
		nombre: paraTraducir("Abajo de la biblioteca"),
		hacer: conCarta((id) => op.aZona(id, "biblioteca", sitio.abajo)),
	},
	{
		contexto: "carta",
		tecla: "i",
		nombre: paraTraducir("Inspeccionar"),
		hacer: conCarta(op.inspeccionar),
	},
	{
		contexto: "carta",
		tecla: "p",
		nombre: paraTraducir("Resaltar"),
		hacer: conCarta(op.alternarResaltado),
	},
	{
		contexto: "carta",
		tecla: "a",
		nombre: paraTraducir("Adjuntar a…"),
		hacer: conCarta(op.adjuntarA),
	},
	{
		contexto: "carta",
		tecla: "d",
		nombre: paraTraducir("Atacar / objetivo"),
		hacer: conCarta((id) => op.flecha(id)),
	},
	{
		contexto: "carta",
		tecla: "o",
		nombre: paraTraducir("Sacar una del montón"),
		hacer: conCarta((id) => op.sacarDelMonton(id)),
	},
	// ── Biblioteca (con el ratón encima: la del tablero grande, que en red puede
	// ser la de otro, como en su menú) ──
	{
		contexto: "biblioteca",
		tecla: "d",
		nombre: paraTraducir("Robar"),
		hacer: () => op.robar(1, duenoBiblioteca()),
	},
	{
		contexto: "biblioteca",
		tecla: "s",
		nombre: paraTraducir("Escrutar"),
		hacer: () => op.mirar("escrutar", "arriba", duenoBiblioteca()),
	},
	{
		contexto: "biblioteca",
		tecla: "u",
		nombre: paraTraducir("Vigilar"),
		hacer: () => op.mirar("vigilar", "arriba", duenoBiblioteca()),
	},
	{
		contexto: "biblioteca",
		tecla: "m",
		nombre: paraTraducir("Moler"),
		hacer: () => op.moler(1, duenoBiblioteca()),
	},
	{
		contexto: "biblioteca",
		tecla: "e",
		nombre: paraTraducir("Exiliar la de arriba"),
		hacer: () => op.exiliarDeArriba("normal", 1, duenoBiblioteca()),
	},
	{
		contexto: "biblioteca",
		tecla: "Mayús+E",
		nombre: paraTraducir("Exiliar la de arriba boca abajo"),
		hacer: () => op.exiliarDeArriba("bocaAbajo", 1, duenoBiblioteca()),
	},
	{
		contexto: "biblioteca",
		tecla: "v",
		nombre: paraTraducir("Ver la biblioteca"),
		hacer: () => op.buscar("todo", duenoBiblioteca()),
	},
	{
		contexto: "biblioteca",
		tecla: "t",
		nombre: paraTraducir("Buscar una carta"),
		hacer: () => op.buscar("todo", duenoBiblioteca()),
	},
	{
		contexto: "biblioteca",
		tecla: "l",
		nombre: paraTraducir("Buscar una tierra"),
		hacer: () => op.buscar("tierra", duenoBiblioteca()),
	},
	{
		contexto: "biblioteca",
		tecla: "h",
		nombre: paraTraducir("Barajar"),
		hacer: () => op.barajar(duenoBiblioteca()),
	},
	{
		contexto: "biblioteca",
		tecla: "o",
		nombre: paraTraducir("Mirar la de arriba"),
		hacer: () => op.mirar("mirar", "arriba", duenoBiblioteca()),
	},
	// ── Toda la mesa ──
	{
		contexto: "mesa",
		tecla: "u",
		nombre: paraTraducir("Enderezar todo"),
		hacer: () => op.enderezarTodo(),
	},
	{
		contexto: "mesa",
		tecla: "Ctrl+z",
		nombre: paraTraducir("Deshacer"),
		hacer: () => op.deshacer(),
	},
	{ contexto: "mesa", tecla: "Ctrl+y", nombre: paraTraducir("Rehacer"), hacer: () => op.rehacer() },
	{
		contexto: "mesa",
		tecla: "Mayús+R",
		nombre: paraTraducir("Responder / volver al turno"),
		hacer: () => void alternarRespuesta(),
	},
	{
		contexto: "mesa",
		tecla: "Ctrl+Enter",
		nombre: paraTraducir("Pasar el turno"),
		hacer: () => op.pasarTurno(),
	},
	{
		contexto: "mesa",
		tecla: "Ctrl+a",
		nombre: paraTraducir("Elegir todo el campo"),
		hacer: () => {
			const m = useMesa.getState();
			const yo = m.yo ?? 0;
			m.seleccionar(m.zonas[`${yo}:campo`]?.cartas ?? []);
		},
	},
	// ── Chat, avisos y señales (los maneja `chat.tsx`: tecla por código físico) ──
	{
		contexto: "mesa",
		tecla: "Enter",
		nombre: paraTraducir("Escribir en el chat"),
		hacer: () => usePantalla.getState().setPanel("chat"),
	},
	...AVISOS.map(
		(a): Atajo => ({
			contexto: "mesa",
			tecla: a.tecla,
			nombre: a.texto,
			hacer: () => void hacer({ que: "aviso", tipo: a.tipo }),
		}),
	),
	...SENALES.map(
		(s): Atajo => ({
			contexto: "mesa",
			tecla: s.tecla,
			nombre: s.atajo,
			hacer: () => {
				const m = useMesa.getState();
				const yo = m.yo;
				if (yo === null) return;
				const nueva = senalTrasElegir(m.jugadores[yo]?.senal ?? null, s.senal);
				void hacer({ que: "senal", senal: nueva });
			},
		}),
	),
	{
		contexto: "mesa",
		tecla: "Escape",
		nombre: paraTraducir("Soltar la selección"),
		hacer: () => {
			useMesa.getState().seleccionar([]);
			usePantalla.getState().setModo(null);
		},
	},
	{
		contexto: "mesa",
		tecla: "?",
		nombre: paraTraducir("Atajos de teclado"),
		hacer: () => usePantalla.getState().abrir({ tipo: "atajos" }),
	},
	{
		contexto: "mesa",
		tecla: "Mayús+?",
		nombre: paraTraducir("Atajos de teclado"),
		hacer: () => usePantalla.getState().abrir({ tipo: "atajos" }),
	},
];

/** Cómo se escribe la tecla de un evento en la tabla. */
export function teclaDe(
	e: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "shiftKey">,
): string {
	const letra = e.key.length === 1 ? e.key.toLowerCase() : e.key;
	const mods =
		(e.ctrlKey || e.metaKey ? "Ctrl+" : "") +
		(e.shiftKey && e.key.length === 1 && /[a-z]/i.test(e.key) ? "Mayús+" : "");
	// Mayús con letra se escribe con la letra en mayúscula.
	return mods.includes("Mayús+") ? `${mods}${letra.toUpperCase()}` : `${mods}${letra}`;
}

/** La tecla de una acción para enseñarla en un menú. */
export function teclaPara(contexto: Contexto, nombre: string): string | undefined {
	const a = ATAJOS.find((x) => x.contexto === contexto && x.nombre === nombre);
	return a ? bonita(a.tecla) : undefined;
}

export function bonita(tecla: string): string {
	return tecla.length === 1 ? tecla.toUpperCase() : tecla;
}

/**
 * El atajo de una tecla: primero el de la carta (si hay una bajo el ratón o
 * elegida), luego el de la biblioteca (con el ratón encima) y luego el de la
 * mesa. 1–9 sobre la biblioteca roba esas.
 */
export function atajoDe(
	tecla: string,
	carta: IdCarta | null,
	sobreBiblioteca: boolean,
): (() => void) | null {
	if (sobreBiblioteca && /^[1-9]$/.test(tecla))
		return () => void op.robar(Number(tecla), duenoBiblioteca());
	const buscar = (c: Contexto) => ATAJOS.find((a) => a.contexto === c && a.tecla === tecla);
	const a =
		(carta !== null ? buscar("carta") : undefined) ??
		(sobreBiblioteca ? buscar("biblioteca") : undefined) ??
		buscar("mesa");
	if (!a) return null;
	return () => a.hacer(carta);
}
