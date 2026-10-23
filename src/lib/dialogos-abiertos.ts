import { useEffect, useRef } from "react";

/**
 * Los diálogos que hay abiertos y lo que cada uno sabe de sí mismo: si tiene
 * un proceso en curso o algo escrito sin guardar (lo mismo que le dice a
 * `DialogContent` con `ocupado` y `cambiosSinGuardar`). Cerrar la app pregunta
 * por todos ellos (`lib/cierre.ts`).
 *
 * Lo declara cada diálogo que lo necesita —con `useEstadoParaCerrar`, en el
 * componente que pinta su `DialogContent`— y no el propio `DialogContent`:
 * `components/ui/` es de solo lectura (heredado de judlex).
 */
export type EstadoDialogo = { enCurso: boolean; sinGuardar: boolean };

const abiertos = new Set<() => EstadoDialogo>();

/** Apunta un diálogo abierto; devuelve cómo quitarlo al cerrarse. */
export function registrarDialogo(estado: () => EstadoDialogo): () => void {
	abiertos.add(estado);
	return () => {
		abiertos.delete(estado);
	};
}

/** Lo que dicen los diálogos abiertos, juntado. */
export function estadoDeLosDialogos(): EstadoDialogo {
	let enCurso = false;
	let sinGuardar = false;
	for (const estado of abiertos) {
		const e = estado();
		enCurso ||= e.enCurso;
		sinGuardar ||= e.sinGuardar;
	}
	return { enCurso, sinGuardar };
}

/**
 * Declara el estado de un diálogo mientras está montado (su `Contenido` solo
 * existe abierto). Llámalo con lo mismo que pasas a `DialogContent`.
 */
export function useEstadoParaCerrar(estado: EstadoDialogo) {
	const actual = useRef(estado);
	actual.current = estado;
	useEffect(() => registrarDialogo(() => actual.current), []);
}
