import { listen } from "@tauri-apps/api/event";
import { guardarAhora } from "@/stores/deck-store";
import { useEdicion } from "@/stores/edicion-store";

/**
 * **Irse a segundo plano en Android no avisa antes de que el sistema congele o
 * mate la app**, y en móvil no hay «cerrar» ni pregunta. Así que al irse se
 * guarda ya todo lo que se tenga en pantalla: el deck y la carta que se está
 * diseñando. Rust guarda la partida (`src-tauri/src/ciclo.rs`).
 *
 * Se entera por `visibilitychange` (lo primero que llega, antes de que el JS
 * pueda quedarse parado) y, por si acaso, por el evento `ciclo://pausa` que
 * manda Rust al recibir `WindowEvent::Suspended`.
 */
export async function guardarTodo(): Promise<void> {
	await Promise.allSettled([guardarAhora(), useEdicion.getState().pendiente?.guardar()]);
}

/** Engancha el guardado al irse. Devuelve cómo soltarlo. */
export function engancharCicloDeVida(): () => void {
	const alCambiar = () => {
		if (document.visibilityState === "hidden") void guardarTodo();
	};
	document.addEventListener("visibilitychange", alCambiar);
	let soltar: (() => void) | null = null;
	let fuera = false;
	void listen("ciclo://pausa", () => void guardarTodo()).then((f) => {
		if (fuera) f();
		else soltar = f;
	});
	return () => {
		fuera = true;
		document.removeEventListener("visibilitychange", alCambiar);
		soltar?.();
	};
}
