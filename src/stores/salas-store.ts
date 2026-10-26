import { create } from "zustand";
import type { ComoEnSala } from "@/bindings/ComoEnSala";
import type { EnlaceSala } from "@/bindings/EnlaceSala";
import type { EstadoSala } from "@/bindings/EstadoSala";
import type { Playgroup } from "@/bindings/Playgroup";

/**
 * **Playgroups y salas del servidor** en pantalla (Fase F). Lo llena
 * `features/salas/conexion-salas.ts` con lo que dice Rust (`salas_*` y los
 * avisos `salas://…`); aquí no se decide nada.
 */
type SalasStore = {
	hayServidor: boolean;
	conectado: boolean;
	/** La clave de esta instalación. */
	yo: string;
	playgroups: Playgroup[];
	sala: EstadoSala | null;
	como: ComoEnSala | null;
	/** El deck preparado para la sala. */
	deck: string | null;
	/** Lo que esta app mide hacia cada otro de la sala. */
	enlaces: Record<string, EnlaceSala>;
	/**
	 * El código de un enlace `proxybuilder://unirse/…` que acaba de abrir la
	 * app: «Jugar con amigos» lo pone en «Unirse con un código» y espera a que
	 * la persona pulse «Unirse».
	 */
	enlaceEntrante: string | null;
	poner: (c: Partial<Omit<SalasStore, "poner" | "sinSala">>) => void;
	sinSala: () => void;
};

export const useSalas = create<SalasStore>((set) => ({
	hayServidor: false,
	conectado: false,
	yo: "",
	playgroups: [],
	sala: null,
	como: null,
	deck: null,
	enlaces: {},
	enlaceEntrante: null,
	poner: (c) => set(c),
	sinSala: () => set({ sala: null, como: null, deck: null, enlaces: {} }),
}));
