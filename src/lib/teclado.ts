/**
 * El teclado de la pantalla no tape lo que se escribe (plan del móvil, M2.3).
 *
 * En la vista web de Android el teclado solo encoge el `visualViewport`: ni
 * `100%` ni `dvh` se enteran. Así que se mide el visor y se deja en `<html>`:
 *
 * - `--alto-visible`: el alto del visor en px, que el shell usa para su alto;
 * - `data-teclado`: presente mientras el teclado está abierto (la barra de
 *   pestañas de abajo se esconde para no quitar sitio).
 *
 * La cuenta (`crearVigilaTeclado`) es pura y se prueba sin pantalla.
 */

/** Lo que tiene que haber encogido el visor para decir que hay un teclado, en px. */
export const UMBRAL_TECLADO = 120;

export interface MedidasVisor {
	/** `window.innerWidth`. */
	ancho: number;
	/** `window.innerHeight`. */
	altoVentana: number;
	/** `visualViewport.height`. */
	altoVisual: number;
	/** `visualViewport.offsetTop`. */
	arriba: number;
	/** Si hay un campo de texto con el foco (sin él no puede haber teclado).
	 *  Sin dato, como si lo hubiera. */
	editando?: boolean;
}

export interface EstadoVisor {
	abierto: boolean;
	/** El alto que queda a la vista, en px. */
	altoVisible: number;
}

/**
 * Recuerda el mayor alto que ha tenido la ventana con este ancho (el de sin
 * teclado) y dice cuánto ha encogido el visor respecto a él. Girar la pantalla
 * (cambia el ancho) o cambiar mucho el alto de la ventana lo reinicia.
 */
export function crearVigilaTeclado() {
	let ancho = 0;
	let altoVentana = 0;
	let mayor = 0;
	return (m: MedidasVisor): EstadoVisor => {
		// Girar (cambia el ancho) vuelve a medir. Si cambia el alto de la ventana
		// sin nada que escribir (pantalla dividida, ventana flotante), también:
		// con un campo con el foco puede ser el propio teclado encogiéndola.
		const otraVentana = m.altoVentana !== altoVentana && m.editando === false;
		if (m.ancho !== ancho || otraVentana) {
			ancho = m.ancho;
			altoVentana = m.altoVentana;
			mayor = 0;
		}
		mayor = Math.max(mayor, m.altoVentana, m.altoVisual + m.arriba);
		const tapado = mayor - (m.altoVisual + m.arriba);
		const abierto = tapado > UMBRAL_TECLADO && m.editando !== false;
		return {
			abierto,
			// Con el teclado, desde arriba hasta donde empieza; sin él, todo.
			altoVisible: abierto ? Math.max(0, m.altoVisual + m.arriba) : m.altoVentana,
		};
	};
}

/**
 * Los campos que se traen a la vista al salir el teclado. Un `textarea` no:
 * el de la lista de cartas crece con el texto (sin scroll propio) y el
 * navegador ya lleva el cursor a la vista al escribir; forzar el campo entero
 * lo mandaría al principio.
 */
const CAMPO = "input, [contenteditable='true']";

/** Engancha el visor a `<html>`. Devuelve cómo soltarlo. */
export function engancharTeclado(): () => void {
	const visor = typeof window === "undefined" ? undefined : window.visualViewport;
	if (!visor) return () => {};
	const raiz = document.documentElement;
	const vigila = crearVigilaTeclado();
	let cuadro = 0;
	let abiertoAntes = false;

	const medir = () => {
		cuadro = 0;
		const e = vigila({
			ancho: window.innerWidth,
			altoVentana: window.innerHeight,
			altoVisual: visor.height,
			arriba: visor.offsetTop,
			editando:
				document.activeElement instanceof HTMLElement &&
				document.activeElement.matches("input, textarea, select, [contenteditable='true']"),
		});
		raiz.style.setProperty("--alto-visible", `${e.altoVisible}px`);
		if (e.abierto) raiz.dataset.teclado = "";
		else delete raiz.dataset.teclado;
		// Al abrirse el teclado, el campo que se escribe se trae a la vista.
		if (e.abierto && !abiertoAntes) {
			const campo = document.activeElement;
			if (campo instanceof HTMLElement && campo.matches(CAMPO)) {
				campo.scrollIntoView({ block: "nearest" });
			}
		}
		abiertoAntes = e.abierto;
	};
	const pedir = () => {
		if (cuadro === 0) cuadro = requestAnimationFrame(medir);
	};
	visor.addEventListener("resize", pedir);
	visor.addEventListener("scroll", pedir);
	window.addEventListener("orientationchange", pedir);
	medir();
	return () => {
		visor.removeEventListener("resize", pedir);
		visor.removeEventListener("scroll", pedir);
		window.removeEventListener("orientationchange", pedir);
		if (cuadro !== 0) cancelAnimationFrame(cuadro);
		raiz.style.removeProperty("--alto-visible");
		delete raiz.dataset.teclado;
	};
}
