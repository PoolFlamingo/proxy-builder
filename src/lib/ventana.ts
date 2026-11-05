import { getCurrentWindow } from "@tauri-apps/api/window";
import { create } from "zustand";

/**
 * **La ventana de escritorio**: pantalla completa con F11 desde cualquier
 * pantalla y los controles propios (minimizar, salir, cerrar) que la barra de
 * arriba enseña cuando el sistema esconde los suyos.
 *
 * El estado no se supone: se lee de la ventana. Sale de pantalla completa
 * también el gestor de ventanas (un atajo suyo, un doble clic) y `window-state`
 * la restaura al arrancar, así que se vuelve a leer en cada cambio de tamaño
 * (como mucho una vez por fotograma) y los controles aparecen cuando la
 * ventana ya es la grande, no antes.
 *
 * 🛑 Cerrar pasa por `close()`, no por `destroy()`: así lo atiende
 * `ConfirmarCierre` como al pulsar la X del sistema (guardar, preguntar).
 */
type Estado = {
	/** La ventana está a pantalla completa ahora mismo. */
	completa: boolean;
};

export const useVentana = create<Estado>(() => ({ completa: false }));

/** Pregunta a la ventana si está a pantalla completa y lo apunta. */
async function leer(): Promise<void> {
	try {
		const completa = await getCurrentWindow().isFullscreen();
		if (completa !== useVentana.getState().completa) useVentana.setState({ completa });
	} catch {
		/* fuera de Tauri o sin permiso: se queda como estaba */
	}
}

// Los cambios de pantalla completa van uno detrás de otro: con F11 pulsado dos
// veces seguidas, el segundo parte de lo que dejó el primero.
let cola: Promise<void> = Promise.resolve();

/** Entra o sale de pantalla completa (F11, los botones de la mesa y de la barra). */
export function alternarPantallaCompleta(): Promise<void> {
	cola = cola.then(async () => {
		try {
			const v = getCurrentWindow();
			await v.setFullscreen(!(await v.isFullscreen()));
		} catch {
			/* sin permiso o fuera de Tauri */
		}
		await leer();
	});
	return cola;
}

/** Minimiza la ventana. */
export async function minimizarVentana(): Promise<void> {
	try {
		await getCurrentWindow().minimize();
	} catch {
		/* fuera de Tauri */
	}
}

/** Pide cerrar la ventana, como la X del sistema (`ConfirmarCierre` decide). */
export async function cerrarVentana(): Promise<void> {
	try {
		await getCurrentWindow().close();
	} catch {
		/* fuera de Tauri */
	}
}

/** ¿Es F11 solo, sin modificadores? (Con Ctrl o Alt es de otra cosa.) */
export function esTeclaPantallaCompleta(
	e: Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "metaKey" | "shiftKey">,
): boolean {
	return e.key === "F11" && !e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey;
}

/**
 * macOS tiene su propia pantalla completa, con los botones del sistema que
 * bajan al acercar el ratón al borde: ahí no se pintan los de la app.
 */
export function esMac(): boolean {
	return typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent);
}

let iniciada = false;

/**
 * Lo que va una sola vez, al arrancar en escritorio (`main.tsx`): leer cómo
 * quedó la ventana, seguir sus cambios de tamaño y enganchar F11.
 *
 * El `keydown` va en captura y en `window` desde antes de montar nada: ningún
 * diálogo, campo ni capa de la mesa lo frena, y funciona con el foco donde sea.
 */
export function iniciarVentana(): void {
	if (iniciada) return;
	iniciada = true;
	void leer();

	let pendiente = false;
	window.addEventListener("resize", () => {
		if (pendiente) return;
		pendiente = true;
		requestAnimationFrame(() => {
			pendiente = false;
			void leer();
		});
	});

	window.addEventListener(
		"keydown",
		(e) => {
			if (e.key !== "F11") return;
			// Siempre se anula: ni la vista web ni el sistema hacen su propia pantalla
			// completa por encima. Pulsada sin soltar no repite (parpadearía).
			e.preventDefault();
			if (e.repeat || !esTeclaPantallaCompleta(e)) return;
			void alternarPantallaCompleta();
		},
		true,
	);
}
