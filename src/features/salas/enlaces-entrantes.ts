import { useEffect } from "react";
import { toast } from "sonner";
import { paraTraducir, traduccionSuelta } from "@/components/language-provider";
import { call, on } from "@/lib/ipc";
import { useMesa } from "@/stores/mesa-store";
import { useSalas } from "@/stores/salas-store";

/**
 * Llega un enlace `proxybuilder://unirse/<código>` (Rust ya validó el código).
 * Con una partida en marcha no se hace nada y se avisa; si no, el código queda
 * en el almacén y la pestaña «Jugar» abre «Jugar con amigos» con él puesto.
 * Devuelve si se aceptó.
 */
export function aceptarEnlace(codigo: string): boolean {
	if (useMesa.getState().activa) {
		toast(
			traduccionSuelta(paraTraducir("Hay una partida en marcha: sal de ella para usar el enlace.")),
		);
		return false;
	}
	useSalas.getState().poner({ enlaceEntrante: codigo });
	return true;
}

/**
 * Escucha los enlaces de invitación (el que abrió la app, que Rust guardó, y
 * los que lleguen después) y lleva a la pestaña «Jugar» con `irAMesa`.
 */
export function useEnlacesEntrantes(irAMesa: () => void) {
	useEffect(() => {
		let soltar: (() => void) | undefined;
		let vivo = true;
		const llega = (codigo: string) => {
			if (aceptarEnlace(codigo)) irAMesa();
		};
		void on("salas://enlace", llega).then((f) => {
			if (vivo) soltar = f;
			else f();
		});
		call("enlace_pendiente")
			.then((c) => c && vivo && llega(c))
			.catch(() => {});
		return () => {
			vivo = false;
			soltar?.();
		};
	}, []);
}
