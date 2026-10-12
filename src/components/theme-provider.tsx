import { createContext, type ReactNode, useContext, useEffect, useMemo, useState } from "react";
import { type Mode, THEMES, type ThemeName } from "@/lib/themes";

// Tema por defecto al abrir por primera vez (hasta que el usuario lo cambie en
// Ajustes): "Amber Minimal" en oscuro, lo más cercano al dorado sobre negro de
// fabricard.net, que es la referencia visual de la app. Si no existiera, cae a
// otro disponible.
//
// Base sigue siendo el tema «limpio» de comparación (`limpio = theme ===
// "base"`, más abajo apaga skin/fondo/material/sombra).
const DEFAULT_THEME: ThemeName = THEMES.some((t) => t.name === "amber-minimal")
	? "amber-minimal"
	: (THEMES[0]?.name ?? ("twitter" as ThemeName));
const DEFAULT_SCALE = 1;
const DEFAULT_MATERIAL: Material = "matte";

export type Skin = "none" | "zeus" | "hera";
export type Background = "none" | "gradient" | "aurora";
export type Material = "solid" | "glass" | "matte";

const DEFAULT_BLUR = 25; // px
const DEFAULT_NOISE = 0.25; // 0..1
const DEFAULT_SHADOW = 3; // 0..4 (Fuerte)

type ThemeState = {
	theme: ThemeName;
	mode: Mode;
	resolvedMode: "light" | "dark";
	scale: number;
	skin: Skin;
	bg: Background;
	material: Material;
	blur: number;
	noise: number;
	shadow: number;
	fxSidebar: boolean;
	heraFiligree: boolean;
	smoothScroll: boolean;
	setTheme: (t: ThemeName) => void;
	setMode: (m: Mode) => void;
	setScale: (s: number) => void;
	setSkin: (s: Skin) => void;
	setBg: (b: Background) => void;
	setMaterial: (m: Material) => void;
	setBlur: (n: number) => void;
	setNoise: (n: number) => void;
	setShadow: (n: number) => void;
	setFxSidebar: (b: boolean) => void;
	setHeraFiligree: (b: boolean) => void;
	setSmoothScroll: (b: boolean) => void;
};

const ThemeContext = createContext<ThemeState | null>(null);

const THEME_KEY = "proxybuilder.theme";
const MODE_KEY = "proxybuilder.mode";
const SCALE_KEY = "proxybuilder.scale";
const SKIN_KEY = "proxybuilder.skin";
const BG_KEY = "proxybuilder.bg";
const MATERIAL_KEY = "proxybuilder.material";
const BLUR_KEY = "proxybuilder.blur";
const NOISE_KEY = "proxybuilder.noise";
const SHADOW_KEY = "proxybuilder.shadow";
const FXSIDEBAR_KEY = "proxybuilder.fxSidebar";
const FILIGREE_KEY = "proxybuilder.heraFiligree";
const SMOOTH_KEY = "proxybuilder.smoothScroll";

/**
 * Un número guardado, o el de fábrica si no hay nada guardado o no vale.
 *
 * 🛑 `Number(localStorage.getItem(k))` NO sirve para esto: sin nada guardado
 * da `Number(null) === 0`, que es un número válido, y el valor de fábrica no
 * se aplicaba nunca (en judlex lo tapaba una migración de una sola vez).
 */
function numeroGuardado(clave: string, defecto: number, valido: (n: number) => boolean): number {
	const crudo = localStorage.getItem(clave);
	if (crudo === null || crudo.trim() === "") return defecto;
	const n = Number(crudo);
	return Number.isFinite(n) && valido(n) ? n : defecto;
}

function prefersDark() {
	return typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function ThemeProvider({ children }: { children: ReactNode }) {
	const [theme, setThemeState] = useState<ThemeName>(() => {
		const stored = localStorage.getItem(THEME_KEY) as ThemeName | null;
		return stored && THEMES.some((t) => t.name === stored) ? stored : DEFAULT_THEME;
	});
	const [mode, setModeState] = useState<Mode>(
		() => (localStorage.getItem(MODE_KEY) as Mode) || "dark",
	);
	const [scale, setScaleState] = useState<number>(() =>
		numeroGuardado(SCALE_KEY, DEFAULT_SCALE, (s) => s > 0),
	);
	const [skin, setSkinState] = useState<Skin>(
		() => (localStorage.getItem(SKIN_KEY) as Skin) || "none",
	);
	const [bg, setBgState] = useState<Background>(
		() => (localStorage.getItem(BG_KEY) as Background) || "none",
	);
	const [material, setMaterialState] = useState<Material>(
		() => (localStorage.getItem(MATERIAL_KEY) as Material) || DEFAULT_MATERIAL,
	);
	const [blur, setBlurState] = useState<number>(() =>
		numeroGuardado(BLUR_KEY, DEFAULT_BLUR, (n) => n >= 0),
	);
	const [noise, setNoiseState] = useState<number>(() =>
		numeroGuardado(NOISE_KEY, DEFAULT_NOISE, (n) => n >= 0),
	);
	const [shadow, setShadowState] = useState<number>(() =>
		numeroGuardado(SHADOW_KEY, DEFAULT_SHADOW, (n) => n >= 0 && n <= 4),
	);
	const [fxSidebar, setFxSidebarState] = useState<boolean>(
		() => localStorage.getItem(FXSIDEBAR_KEY) === "1",
	);
	const [heraFiligree, setHeraFiligreeState] = useState<boolean>(
		() => localStorage.getItem(FILIGREE_KEY) === "1",
	);
	// Scroll suave (SmoothScroll de base/web, no Lenis): activado por defecto.
	const [smoothScroll, setSmoothScrollState] = useState<boolean>(
		() => localStorage.getItem(SMOOTH_KEY) !== "0",
	);
	const [systemDark, setSystemDark] = useState(prefersDark);

	// Track OS appearance changes when mode === "system".
	useEffect(() => {
		const mq = window.matchMedia("(prefers-color-scheme: dark)");
		const onChange = () => setSystemDark(mq.matches);
		mq.addEventListener("change", onChange);
		return () => mq.removeEventListener("change", onChange);
	}, []);

	const resolvedMode: "light" | "dark" = mode === "system" ? (systemDark ? "dark" : "light") : mode;

	// Apply theme + appearance to <html>.
	useEffect(() => {
		const root = document.documentElement;
		root.setAttribute("data-theme", theme);
		root.classList.toggle("dark", resolvedMode === "dark");
	}, [theme, resolvedMode]);

	// Escala global de la interfaz.
	//
	// NUNCA con `zoom`: en WKWebView rompe `position: fixed` y descoloca el
	// cálculo de posición de Radix (que mide con getBoundingClientRect). El
	// resultado eran modales descentrados, el velo sin cubrir la pantalla y los
	// desplegables cayendo lejos de su campo. Con el tamaño de fuente de la raíz
	// escala todo igual (Tailwind trabaja en rem) y la geometría sigue siendo
	// correcta. MacUp no lo detectó porque no usa diálogos.
	useEffect(() => {
		const raiz = document.documentElement;
		(raiz.style as CSSStyleDeclaration & { zoom?: string }).removeProperty("zoom");
		raiz.style.fontSize = scale === 1 ? "" : `${Math.round(scale * 100)}%`;
	}, [scale]);

	/**
	 * 🛑 El tema **Base** se pinta LIMPIO: sin skin, sin fondo, sin material y
	 * sin efectos.
	 *
	 * No es un capricho de estilo, es lo que hace que sirva de referencia. Base
	 * es el tema por defecto de shadcn tal cual sale de la caja, y está en la
	 * lista precisamente para poder comparar: cuando algo se ve raro en un tema
	 * de color, se cambia a Base y se sabe si el problema es del tema o del
	 * componente. Con el cristal de Hera encima, o con una aurora de fondo, esa
	 * comparación no vale para nada.
	 *
	 * Lo que el usuario tenga elegido NO se borra: se guarda igual y vuelve en
	 * cuanto elige otro tema. Aquí solo se deja de aplicar.
	 */
	const limpio = theme === "base";

	// Apply surface skin (Zeus / Hera) to <html>.
	useEffect(() => {
		const root = document.documentElement;
		if (limpio || skin === "none") root.removeAttribute("data-skin");
		else root.setAttribute("data-skin", skin);
	}, [skin, limpio]);

	// Apply background + card material options to <html>.
	useEffect(() => {
		const root = document.documentElement;
		if (limpio || bg === "none") root.removeAttribute("data-bg");
		else root.setAttribute("data-bg", bg);
	}, [bg, limpio]);

	useEffect(() => {
		const root = document.documentElement;
		if (limpio || material === "solid") root.removeAttribute("data-material");
		else root.setAttribute("data-material", material);
	}, [material, limpio]);

	// Blur strength + noise opacity as CSS vars (driven by the sliders).
	useEffect(() => {
		const s = document.documentElement.style;
		s.setProperty("--macup-blur", `${blur}px`);
		s.setProperty("--macup-noise", String(noise));
	}, [blur, noise]);

	// Shadow level (0..4) as a CSS var so every surface (incl. skins) reacts.
	useEffect(() => {
		const SHADOWS = [
			"none",
			"0 1px 2px rgba(0,0,0,.10), 0 1px 3px rgba(0,0,0,.08)",
			"0 4px 10px -2px rgba(0,0,0,.16), 0 2px 4px -2px rgba(0,0,0,.12)",
			"0 12px 26px -6px rgba(0,0,0,.30), 0 4px 8px -4px rgba(0,0,0,.18)",
			"0 22px 48px -10px rgba(0,0,0,.45), 0 8px 18px -8px rgba(0,0,0,.30)",
		];
		document.documentElement.style.setProperty(
			"--macup-shadow",
			limpio ? "none" : (SHADOWS[shadow] ?? SHADOWS[2]),
		);
	}, [shadow, limpio]);

	// Extend card effects (glass/matte) to the sidebar + header when enabled.
	useEffect(() => {
		const root = document.documentElement;
		if (fxSidebar && !limpio) root.setAttribute("data-fx-sidebar", "1");
		else root.removeAttribute("data-fx-sidebar");
	}, [fxSidebar, limpio]);

	// Filigrana (hairlines finas) sobre el skin Hera cuando se activa.
	useEffect(() => {
		const root = document.documentElement;
		if (heraFiligree && !limpio) root.setAttribute("data-hera-filigree", "1");
		else root.removeAttribute("data-hera-filigree");
	}, [heraFiligree, limpio]);

	const value = useMemo<ThemeState>(
		() => ({
			theme,
			mode,
			resolvedMode,
			scale,
			skin,
			bg,
			material,
			blur,
			noise,
			shadow,
			fxSidebar,
			heraFiligree,
			smoothScroll,
			setTheme: (t) => {
				setThemeState(t);
				localStorage.setItem(THEME_KEY, t);
			},
			setMode: (m) => {
				setModeState(m);
				localStorage.setItem(MODE_KEY, m);
			},
			setScale: (s) => {
				setScaleState(s);
				localStorage.setItem(SCALE_KEY, String(s));
			},
			setSkin: (s) => {
				setSkinState(s);
				localStorage.setItem(SKIN_KEY, s);
			},
			setBg: (b) => {
				setBgState(b);
				localStorage.setItem(BG_KEY, b);
			},
			setMaterial: (m) => {
				setMaterialState(m);
				localStorage.setItem(MATERIAL_KEY, m);
			},
			setBlur: (n) => {
				setBlurState(n);
				localStorage.setItem(BLUR_KEY, String(n));
			},
			setNoise: (n) => {
				setNoiseState(n);
				localStorage.setItem(NOISE_KEY, String(n));
			},
			setShadow: (n) => {
				setShadowState(n);
				localStorage.setItem(SHADOW_KEY, String(n));
			},
			setFxSidebar: (b) => {
				setFxSidebarState(b);
				localStorage.setItem(FXSIDEBAR_KEY, b ? "1" : "0");
			},
			setHeraFiligree: (b) => {
				setHeraFiligreeState(b);
				localStorage.setItem(FILIGREE_KEY, b ? "1" : "0");
			},
			setSmoothScroll: (b) => {
				setSmoothScrollState(b);
				localStorage.setItem(SMOOTH_KEY, b ? "1" : "0");
			},
		}),
		[
			theme,
			mode,
			resolvedMode,
			scale,
			skin,
			bg,
			material,
			blur,
			noise,
			shadow,
			fxSidebar,
			heraFiligree,
			smoothScroll,
		],
	);

	return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
	const ctx = useContext(ThemeContext);
	if (!ctx) throw new Error("useTheme must be used within ThemeProvider");
	return ctx;
}
