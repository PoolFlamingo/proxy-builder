import { useEffect } from "react";
import { usePasarUi } from "@/features/pasar/store";
import { call, on } from "@/lib/ipc";

/**
 * **Un `.pbdeck` que otra app abre con esta** (Android, «Abrir con…» desde
 * Archivos, Drive, un mensaje…). Rust (`abrir.rs`) guarda la `content://` y
 * avisa con `pbdeck://abierto`; si la app arrancó con ese fichero, la pantalla
 * aún no escuchaba y lo pide con `pbdeck_pendiente`.
 *
 * Nada se importa sin preguntar: se abre el diálogo de importar (el mismo que
 * al recibir de otro equipo: `features/pasar/importar-dialog.tsx`), que enseña
 * lo que trae, pregunta con o sin proxies y, si ya está, reemplazar o guardar
 * los dos.
 */
export function useAbrirPbdeck() {
	useEffect(() => {
		let vivo = true;
		let soltar: (() => void) | undefined;
		const llega = (uri: string) => usePasarUi.getState().abrirImportar(uri);
		void on("pbdeck://abierto", (uri) => {
			// El aviso ya lo recibimos: lo guardado en Rust no debe repetirse.
			void call("pbdeck_pendiente").catch(() => {});
			llega(uri);
		}).then((f) => {
			if (vivo) soltar = f;
			else f();
		});
		call("pbdeck_pendiente")
			.then((uri) => uri && vivo && llega(uri))
			.catch(() => {});
		return () => {
			vivo = false;
			soltar?.();
		};
	}, []);
}
