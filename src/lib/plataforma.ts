import { useEffect, useState } from "react";
import type { Plataforma } from "@/bindings/Plataforma";
import { call } from "@/lib/ipc";

/**
 * **En qué sistema corre la app**, de verdad (Rust lo sabe: `app_plataforma`),
 * no por el tamaño de la ventana: «solo en móvil» (Compartir, el selector de
 * ficheros de Android…) es Android o iOS, no una ventana estrecha ni una
 * pantalla táctil de portátil. Para decidir la disposición está
 * `useFormato()` (`lib/formato.ts`).
 */
let carga: Promise<Plataforma | null> | null = null;

export function plataforma(): Promise<Plataforma | null> {
	carga ??= Promise.resolve()
		.then(() => call("app_plataforma"))
		.then((p) => p ?? null)
		.catch(() => null);
	return carga;
}

export function esMovilPlataforma(p: Plataforma | null): boolean {
	return p === "android" || p === "ios";
}

/** ¿Es Android o iOS? (`false` fuera de Tauri.) */
export async function esMovil(): Promise<boolean> {
	return esMovilPlataforma(await plataforma());
}

/** Lo mismo, para pintar: `false` hasta que Rust contesta (un instante). */
export function useEsMovil(): boolean {
	const [movil, setMovil] = useState(false);
	useEffect(() => {
		let vivo = true;
		void esMovil().then((m) => {
			if (vivo) setMovil(m);
		});
		return () => {
			vivo = false;
		};
	}, []);
	return movil;
}
