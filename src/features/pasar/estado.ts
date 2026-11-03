import { useEffect, useState } from "react";
import type { EquipoCerca } from "@/bindings/EquipoCerca";
import type { EstadoEmisor } from "@/bindings/EstadoEmisor";
import type { EstadoReceptor } from "@/bindings/EstadoReceptor";
import { type Events, on } from "@/lib/ipc";

/**
 * El estado que Rust tiene de pasar decks, para pintarlo: la pantalla sigue
 * los eventos desde que se monta (Rust emite el estado al abrir «Recibir» y al
 * lanzar un envío, y cerrar la pantalla lo cancela, así que no hay nada
 * anterior que leer). Nunca lo decide ella.
 */
function useEventoRust<E extends keyof Events, T extends Events[E]>(evento: E): T | null {
	const [valor, setValor] = useState<T | null>(null);
	useEffect(() => {
		let vivo = true;
		let soltar: (() => void) | undefined;
		void on(evento, (v) => vivo && setValor(v as T)).then((f) => {
			if (vivo) soltar = f;
			else f();
		});
		return () => {
			vivo = false;
			soltar?.();
		};
	}, [evento]);
	return valor;
}

/** Lo que va de «Recibir». */
export function useReceptor(): EstadoReceptor | null {
	return useEventoRust("pasar://receptor");
}

/** Lo que va del envío en marcha. */
export function useEmisor(): EstadoEmisor | null {
	return useEventoRust("pasar://emisor");
}

/** Los equipos de la red local que esperan un deck. */
export function useCerca(): EquipoCerca[] {
	return useEventoRust<"pasar://cerca", EquipoCerca[]>("pasar://cerca") ?? [];
}
