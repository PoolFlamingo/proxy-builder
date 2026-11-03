/**
 * El botón (o gesto) «atrás» de Android. Orden:
 *
 * 1. Si hay una capa de Radix abierta (diálogo, menú, lista), se cierra la de
 *    arriba con un `Escape`: es lo que Radix ya entiende.
 * 2. Si no, el último manejador registrado que lo atienda (los paneles de la
 *    mesa, Diseñar…): `registrarAtras`, el último registrado va primero. Uno
 *    que no pueda hacer nada devuelve `false` y pasa al siguiente.
 * 3. Si ninguno, el de por defecto (`fijarAtrasPorDefecto`): en la biblioteca,
 *    «pulsa otra vez para salir».
 */
export type ManejadorAtras = () => boolean | undefined;

export interface Entorno {
	capaAbierta(): boolean;
	cerrarCapa(): void;
}

export function crearAtras(entorno: Entorno) {
	const pila: ManejadorAtras[] = [];
	let porDefecto: (() => void) | null = null;
	return {
		registrar(m: ManejadorAtras) {
			pila.push(m);
			return () => {
				const i = pila.lastIndexOf(m);
				if (i >= 0) pila.splice(i, 1);
			};
		},
		fijarPorDefecto(f: (() => void) | null) {
			porDefecto = f;
		},
		/** Atiende un «atrás». Devuelve quién lo atendió. */
		atras(): "capa" | "manejador" | "defecto" | "nadie" {
			if (entorno.capaAbierta()) {
				entorno.cerrarCapa();
				return "capa";
			}
			for (let i = pila.length - 1; i >= 0; i--) {
				if (pila[i]?.() !== false) return "manejador";
			}
			if (porDefecto) {
				porDefecto();
				return "defecto";
			}
			return "nadie";
		},
	};
}

/** Las capas de Radix abiertas (llevan `data-state="open"`); un `role="dialog"`
 *  propio que no sea de Radix —p. ej. la reacción del sparring, donde `Escape`
 *  decide— no cuenta: el «atrás» nunca toma una decisión de juego. */
const CAPAS = ["dialog", "alertdialog", "menu", "listbox"]
	.map((r) => `[role="${r}"][data-state="open"]`)
	.join(", ");

const global = crearAtras({
	capaAbierta: () => typeof document !== "undefined" && !!document.querySelector(CAPAS),
	cerrarCapa: () => {
		// Sobre el foco (Radix lo deja dentro de la capa): así los atajos de la
		// pantalla, que miran si la tecla viene de un diálogo o un menú, no hacen
		// además lo suyo con el `Escape` (soltar la selección).
		const destino = document.activeElement ?? document;
		destino.dispatchEvent(
			new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }),
		);
	},
});

export const registrarAtras = global.registrar;
export const fijarAtrasPorDefecto = global.fijarPorDefecto;
export const atras = global.atras;

/** Engancha el botón «atrás» de Android a `atras()`. Devuelve cómo soltarlo. */
export async function enlazarAtras(): Promise<() => void> {
	const { onBackButtonPress } = await import("@tauri-apps/api/app");
	const oyente = await onBackButtonPress(() => {
		atras();
	});
	return () => {
		void oyente.unregister();
	};
}
