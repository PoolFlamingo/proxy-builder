import { create } from "zustand";

/**
 * **El nombre con el que se juega**: el último que se eligió. Lo comparten la
 * entrada de «Jugar» y la sala de espera de la partida en red, que lo cambia
 * en caliente (los demás lo ven al momento); se recuerda entre sesiones.
 */
const CLAVE = "proxybuilder.mesa.nombre";

/** Lo más largo que puede ser (el mismo tope que pone Rust en la sala). */
export const MAX_NOMBRE = 24;

function leer(): string {
	try {
		return localStorage.getItem(CLAVE) ?? "";
	} catch {
		return "";
	}
}

type Nombre = {
	nombre: string;
	poner: (n: string) => void;
};

export const useNombre = create<Nombre>((set) => ({
	nombre: leer(),
	poner: (nombre) => {
		set({ nombre });
		try {
			localStorage.setItem(CLAVE, nombre);
		} catch {
			/* sin almacenamiento: dura hasta cerrar la app */
		}
	},
}));
