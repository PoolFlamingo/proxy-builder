import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect } from "react";
import { toast } from "sonner";
import { useLang } from "@/components/language-provider";
import { anadirPropias, EXTENSIONES_IMAGEN } from "@/features/decklist/acciones";
import { reemplazarConFichero } from "./acciones-carta";

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
				const rutas = ev.payload.paths.filter((p) =>
					EXTENSIONES_IMAGEN.includes(p.split(".").pop()?.toLowerCase() ?? ""),
				);
				if (rutas.length === 0) return;
				const dpr = window.devicePixelRatio || 1;
				const { x, y } = ev.payload.position;
				const el = document.elementFromPoint(x / dpr, y / dpr);
				const id = el?.closest<HTMLElement>("[data-carta]")?.dataset.carta;
				const hecho = id ? reemplazarConFichero(id, rutas[0]) : anadirPropias(rutas, t);
				hecho.catch((e) => toast.error(mensaje(e)));
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
