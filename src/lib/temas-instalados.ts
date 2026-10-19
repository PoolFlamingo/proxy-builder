import { create } from "zustand";
import type { TemaInstalado } from "@/bindings/TemaInstalado";
import { raizProtocolo } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { THEMES } from "@/lib/themes";

/**
 * **Los temas de tweakcn instalados** (`src-tauri/src/temas.rs`). Su CSS va en
 * un `<style>` propio y cada uno entra al final de `THEMES`. Los dos se
 * recuerdan también aquí (`RECUERDO_KEY`) para ponerlos antes de montar la
 * pantalla (`prepararTemasInstalados`, en `main.tsx`): el `ThemeProvider` solo
 * acepta el tema guardado si está en `THEMES`, y sin eso un tema instalado
 * arrancaría con los colores de serie hasta que contestase Rust.
 */
const RECUERDO_KEY = "proxybuilder.temasInstalados";
const ESTILO_ID = "temas-tweakcn";
/** Lo que Rust pone en las URL de las letras en lugar de la raíz del protocolo. */
const BASE = "{{PBIMG}}";

export function esTemaInstalado(nombre: string): boolean {
	return nombre.startsWith("tw-");
}

function ponerCss(css: string) {
	let estilo = document.getElementById(ESTILO_ID);
	if (!estilo) {
		estilo = document.createElement("style");
		estilo.id = ESTILO_ID;
		document.head.appendChild(estilo);
	}
	estilo.textContent = css;
}

function ponerEnLaLista(temas: TemaInstalado[]) {
	for (let i = THEMES.length - 1; i >= 0; i--) {
		if (esTemaInstalado(THEMES[i].name)) THEMES.splice(i, 1);
	}
	for (const t of temas)
		THEMES.push({ name: t.id as `tw-${string}`, label: t.nombre, swatch: t.muestra });
}

function aplicar(temas: TemaInstalado[]) {
	const raiz = raizProtocolo();
	ponerCss(temas.map((t) => t.css.split(BASE).join(raiz)).join("\n"));
	ponerEnLaLista(temas);
}

function recordar(temas: TemaInstalado[]) {
	try {
		if (temas.length) localStorage.setItem(RECUERDO_KEY, JSON.stringify(temas));
		else localStorage.removeItem(RECUERDO_KEY);
	} catch {
		// Sin almacenamiento: el tema llega igual en cuanto conteste Rust.
	}
}

/** Pone los temas instalados de la última vez. Antes de montar la pantalla. */
export function prepararTemasInstalados() {
	try {
		const crudo = localStorage.getItem(RECUERDO_KEY);
		if (crudo) aplicar(JSON.parse(crudo) as TemaInstalado[]);
	} catch {
		// Recuerdo roto: se queda sin ellos hasta que conteste Rust.
	}
}

type Estado = {
	/** `null` hasta la primera respuesta de Rust. */
	temas: TemaInstalado[] | null;
	cargar: () => Promise<void>;
	/** Rechaza con el error de Rust. */
	instalar: (id: string) => Promise<TemaInstalado>;
	quitar: (id: string) => Promise<void>;
};

export const useTemasInstalados = create<Estado>((set, get) => {
	const poner = (temas: TemaInstalado[]) => {
		aplicar(temas);
		recordar(temas);
		set({ temas });
	};
	return {
		temas: null,
		cargar: async () => {
			try {
				poner(await call("themes_installed", {}));
			} catch {
				set({ temas: get().temas ?? [] });
			}
		},
		instalar: async (id) => {
			const t = await call("theme_install", { id });
			poner(
				[...(get().temas ?? []).filter((x) => x.id !== t.id), t].sort((a, b) =>
					a.nombre.localeCompare(b.nombre),
				),
			);
			return t;
		},
		quitar: async (id) => {
			await call("theme_remove", { id });
			poner((get().temas ?? []).filter((x) => x.id !== id));
		},
	};
});
