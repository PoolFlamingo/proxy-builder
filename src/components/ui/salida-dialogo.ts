import * as React from "react";

/**
 * Lo que un diálogo sabe de cómo se sale de él (ver `DialogContent` en
 * `dialog.tsx`). Vive aparte para que `Formulario` lo pueda usar sin importar
 * el diálogo, que ya importa a `Formulario`.
 */
export interface Salida {
	/** Un formulario o un componente de dentro avisa de que empieza un
	 *  trabajo, con cómo pararlo; lo que devuelve se llama al terminar.
	 *
	 *  🛑 En una ref y no con `useFormStatus`: medido en Chromium, el `pending`
	 *  solo vale `true` en los renders de la transición de la acción; en
	 *  cualquier otro (el progreso que pinta la propia acción tras su primer
	 *  `await`) vuelve a `false`, y el diálogo creía que no había nada en
	 *  marcha. */
	empezarProceso: (interrumpir: () => void) => () => void;
	/** ¿Hay algo en marcha o algo escrito? */
	hayQuePreguntar: () => boolean;
	/** Pregunta si hace falta, y después `salir`. */
	pedirSalida: (salir: () => void) => void;
	/** Cierra el diálogo como lo cerraría la X. */
	cerrar: () => void;
}

export const SalidaDelDialogo = React.createContext<Salida | null>(null);

/**
 * Un trabajo en marcha DENTRO de un diálogo que no va por la acción de un
 * formulario (una búsqueda, una espera): mientras `activo`, salir del diálogo
 * pregunta antes, y si la persona dice que sí se llama a `interrumpir`.
 * Fuera de un diálogo no hace nada. Si el trabajo está en el mismo componente
 * que pinta el `DialogContent`, se dice con su prop `ocupado`.
 */
export function useTrabajoEnElDialogo(activo: boolean, interrumpir?: () => void) {
	const salida = React.useContext(SalidaDelDialogo);
	const parar = React.useRef(interrumpir);
	parar.current = interrumpir;
	React.useEffect(() => {
		if (!activo || !salida) return;
		const terminar = salida.empezarProceso(() => parar.current?.());
		return () => {
			terminar();
		};
	}, [activo, salida]);
}
