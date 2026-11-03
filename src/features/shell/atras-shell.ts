import { toast } from "sonner";
import { guardarTodo } from "@/lib/ciclo-de-vida";
import { call } from "@/lib/ipc";
import type { Vista } from "@/stores/ui-store";

/** Lo que tiene la segunda pulsación de «atrás» para salir, en ms. */
export const VENTANA_SALIR_MS = 2000;

/**
 * «Pulsa otra vez para salir»: la primera pulsación avisa; la segunda, dentro
 * de `ventana` ms, sale. Pura (reloj, aviso y salida se pasan) para probarla
 * sin pantalla.
 */
export function crearSalidaConDobleToque(opciones: {
	ahora: () => number;
	avisar: () => void;
	salir: () => void;
	ventana?: number;
}) {
	const ventana = opciones.ventana ?? VENTANA_SALIR_MS;
	let ultima: number | null = null;
	return () => {
		const ahora = opciones.ahora();
		if (ultima !== null && ahora - ultima <= ventana) {
			ultima = null;
			opciones.salir();
			return;
		}
		ultima = ahora;
		opciones.avisar();
	};
}

/**
 * Qué hace «atrás» cuando no hay nada abierto (después de cerrar la capa de
 * arriba y de los manejadores de cada pantalla): desde Diseñar vuelve a Deck;
 * en cualquier otra pantalla, `null` (le toca el de por defecto, salir con
 * doble toque).
 */
export function destinoDeAtras(vista: Vista): Vista | null {
	return vista === "disenar" ? "deck" : null;
}

/**
 * El «atrás» de por defecto de la app: guarda el deck y la carta y sale de la
 * app (`app_salir`) a la segunda pulsación seguida. `texto` es el aviso ya
 * traducido; `ocupado`, el motivo (traducido) por el que ahora no se puede
 * salir, o `null`.
 */
export function atrasPorDefecto(
	texto: () => string,
	ocupado: () => string | null = () => null,
): () => void {
	const doble = crearSalidaConDobleToque({
		ahora: () => Date.now(),
		avisar: () => {
			toast(texto(), { duration: VENTANA_SALIR_MS });
		},
		salir: () => {
			// Lo mismo que guarda el cierre de escritorio; si algo falla, se sale igual.
			void guardarTodo().finally(() => void call("app_salir").catch(() => {}));
		},
	});
	// Con un trabajo largo o una actualización bajando no se sale: se dice por qué.
	return () => {
		const motivo = ocupado();
		if (motivo) toast(motivo);
		else doble();
	};
}
