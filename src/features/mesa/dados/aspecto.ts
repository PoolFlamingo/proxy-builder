import { create } from "zustand";
import type { Acabado } from "@/bindings/Acabado";
import type { AspectoDados } from "@/bindings/AspectoDados";
import type { Metal } from "@/bindings/Metal";
import type { VistaAspectoDados } from "@/bindings/VistaAspectoDados";
import { call } from "@/lib/ipc";

/**
 * **El aspecto de los dados y la moneda** de quien juega (Ajustes > Mesa).
 * Lo guarda Rust (`mesa/dados.rs`, con la imagen propia en el almacén de
 * blobs); aquí se lee una vez y se convierte en el `Estilo` que pintan los
 * tres niveles. Solo es pintura: el resultado lo decide el motor.
 */

export const ASPECTO_DE_FABRICA: AspectoDados = {
	color: null,
	numeros: "#ffffff",
	acabado: "liso",
	imagen: null,
	moneda: "oro",
};

type Estado = {
	vista: VistaAspectoDados | null;
	cargar: () => Promise<void>;
	poner: (a: AspectoDados) => Promise<void>;
	/** Una imagen del equipo (o ninguna, con `null`). */
	ponerImagen: (ruta: string | null) => Promise<void>;
};

let cargando: Promise<void> | null = null;

export const useAspectoDados = create<Estado>((set) => ({
	vista: null,
	cargar: () => {
		cargando ??= call("mesa_dados_get", {})
			.then((vista) => set({ vista }))
			.catch(() => {
				cargando = null;
			});
		return cargando;
	},
	poner: async (aspecto) => set({ vista: await call("mesa_dados_set", { aspecto }) }),
	ponerImagen: async (ruta) => set({ vista: await call("mesa_dados_imagen", { ruta }) }),
}));

/** Lo que necesita cada nivel para pintar una tirada. */
export type Estilo = {
	color: string;
	numeros: string;
	/** El trazo que rodea los números para que se lean sobre cualquier fondo. */
	contorno: string;
	acabado: Acabado;
	/** La imagen propia (`data:` URL), que tapa el color y el acabado. */
	imagen: string | null;
	moneda: Metal;
};

/** Claro u oscuro (luminancia relativa aproximada de un `#rrggbb`). */
export function esClaro(hex: string): boolean {
	const n = Number.parseInt(hex.slice(1, 7), 16);
	if (Number.isNaN(n)) return true;
	const r = (n >> 16) & 255;
	const g = (n >> 8) & 255;
	const b = n & 255;
	return 0.2126 * r + 0.7152 * g + 0.0722 * b > 140;
}

/**
 * El estilo de una tirada: con el aspecto de quien juega si la tiró él, y si
 * no, el de fábrica con el color de quien tiró. (En línea, cada jugador
 * mandará el suyo.)
 */
export function estiloDe(
	vista: VistaAspectoDados | null,
	colorJugador: string,
	esMia: boolean,
): Estilo {
	const a = esMia && vista ? vista.aspecto : ASPECTO_DE_FABRICA;
	return {
		color: a.color ?? colorJugador,
		numeros: a.numeros,
		contorno: esClaro(a.numeros) ? "rgba(0,0,0,.55)" : "rgba(255,255,255,.6)",
		acabado: a.acabado,
		imagen: esMia && vista ? vista.imagen : null,
		moneda: a.moneda,
	};
}

/** La clave de un estilo (para no repintar su fondo). */
export function claveEstilo(e: Estilo): string {
	return `${e.color}|${e.acabado}|${e.imagen ? e.imagen.length + e.imagen.slice(-32) : ""}`;
}

/** Los colores de cada metal de la moneda. */
export const METALES: Record<
	Metal,
	{ claro: string; oscuro: string; borde: string; canto: string; simbolo: string; trazo: string }
> = {
	oro: {
		claro: "#f8d77a",
		oscuro: "#b8862b",
		borde: "#9a6f1f",
		canto: "#c9992f",
		simbolo: "#ffffff",
		trazo: "rgba(80,50,0,.55)",
	},
	plata: {
		claro: "#f4f6f8",
		oscuro: "#99a2ac",
		borde: "#7b838c",
		canto: "#b9c0c8",
		simbolo: "#5d656e",
		trazo: "rgba(255,255,255,.7)",
	},
	cobre: {
		claro: "#f2b48a",
		oscuro: "#a4532c",
		borde: "#87411f",
		canto: "#b8653a",
		simbolo: "#fff1e6",
		trazo: "rgba(70,25,0,.55)",
	},
};
