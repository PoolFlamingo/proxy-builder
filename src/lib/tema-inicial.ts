import { THEMES } from "@/lib/themes";

/**
 * **El tema puesto antes de montar la pantalla.** El `ThemeProvider` pone el
 * tema, el modo oscuro y la superficie en `<html>` desde un efecto, que va
 * después del primer pintado: hasta entonces la página sale con los colores
 * por defecto (claros) y, con la ventana ya enseñada, se ve un destello
 * blanco. Esto hace lo mismo de una vez, al arrancar.
 *
 * 🛑 Las claves y los valores de fábrica son los de
 * `components/theme-provider.tsx` (de solo lectura, heredado de judlex): si
 * cambian allí, aquí también.
 */
const TEMA_FABRICA = "amber-minimal";

function leer(clave: string): string | null {
	try {
		return localStorage.getItem(clave);
	} catch {
		return null;
	}
}

export function ponerTemaAntesDeMontar() {
	const raiz = document.documentElement;
	const guardado = leer("proxybuilder.theme");
	// Como el provider: solo un tema que esté en `THEMES` (los instalados ya
	// entraron con `prepararTemasInstalados`).
	const tema =
		guardado && THEMES.some((t) => t.name === guardado)
			? guardado
			: THEMES.some((t) => t.name === TEMA_FABRICA)
				? TEMA_FABRICA
				: (THEMES[0]?.name ?? TEMA_FABRICA);
	const modo = leer("proxybuilder.mode") || "dark";
	const oscuro =
		modo === "system"
			? window.matchMedia?.("(prefers-color-scheme: dark)").matches === true
			: modo === "dark";
	raiz.setAttribute("data-theme", tema);
	raiz.classList.toggle("dark", oscuro);
	// La superficie: el tema «base» va limpio, sin skin, fondo ni material.
	if (tema === "base") return;
	const skin = leer("proxybuilder.skin") || "none";
	const fondo = leer("proxybuilder.bg") || "none";
	const material = leer("proxybuilder.material") || "matte";
	if (skin !== "none") raiz.setAttribute("data-skin", skin);
	if (fondo !== "none") raiz.setAttribute("data-bg", fondo);
	if (material !== "solid") raiz.setAttribute("data-material", material);
}
