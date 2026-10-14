import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect } from "react";
import { toast } from "sonner";
import { useLang } from "@/components/language-provider";
import { anadirPropias, EXTENSIONES_IMAGEN, type T } from "@/features/decklist/acciones";
import { reemplazarConFichero } from "./acciones-carta";

/** De lo que se arrastra, solo las imágenes que la app sabe leer. */
export function soloImagenes(rutas: string[]): string[] {
	return rutas.filter((p) => EXTENSIONES_IMAGEN.includes(p.split(".").pop()?.toLowerCase() ?? ""));
}

/**
 * Suelta ficheros en `posicion` (píxeles físicos, como los da Tauri): sobre
 * una carta (`cartaEn`, en píxeles CSS) la reemplaza con la primera imagen;
 * fuera de las cartas, las añade todas como propias. `null` si no hay
 * ninguna imagen.
 */
export function soltarFicheros(
	rutas: string[],
	posicion: { x: number; y: number },
	dpr: number,
	cartaEn: (x: number, y: number) => string | undefined,
	t: T,
): Promise<void> | null {
	const imagenes = soloImagenes(rutas);
	if (imagenes.length === 0) return null;
	const id = cartaEn(posicion.x / dpr, posicion.y / dpr);
	return id ? reemplazarConFichero(id, imagenes[0]) : anadirPropias(imagenes, t);
}

/** El id de la carta que hay en (x, y) de la ventana, si hay alguna. */
function cartaEnPantalla(x: number, y: number): string | undefined {
	return document.elementFromPoint(x, y)?.closest<HTMLElement>("[data-carta]")?.dataset.carta;
}

/**
 * Arrastrar imágenes desde el gestor de ficheros: sobre una carta, la
 * reemplaza (la primera imagen); fuera de las cartas, se añaden todas como
 * cartas propias. Tauri da la posición en píxeles físicos.
 */
export function useArrastrarFicheros() {
	const { t, mensaje } = useLang();
	useEffect(() => {
		let dejar: (() => void) | null = null;
		let vivo = true;
		getCurrentWebview()
			.onDragDropEvent((ev) => {
				if (ev.payload.type !== "drop") return;
				const hecho = soltarFicheros(
					ev.payload.paths,
					ev.payload.position,
					window.devicePixelRatio || 1,
					cartaEnPantalla,
					t,
				);
				hecho?.catch((e) => toast.error(mensaje(e)));
			})
			.then((f) => {
				if (vivo) dejar = f;
				else f();
			})
			.catch(() => {
				/* fuera de Tauri (tests): no hay arrastre de ficheros */
			});
		return () => {
			vivo = false;
			dejar?.();
		};
	}, [t, mensaje]);
}
