// Generado en su día por `scripts/build-themes.mjs` (que vive en MacUp, no en
// este repo). Aquí se edita a mano cuando hace falta, con cuidado de que el
// orden y los nombres casen con `src/themes.css`.
export type Mode = "light" | "dark" | "system";

/** Los temas que trae la app (`src/themes.css`). */
export type TemaDeSerie =
	| "base"
	| "claymorphism"
	| "starry-night"
	| "twitter"
	| "perpetuity"
	| "amber-minimal"
	| "clean-slate"
	| "cosmic-night"
	| "elegant-luxury"
	| "mocha-mousse"
	| "kodama-grove"
	| "nature"
	| "notebook"
	| "ocean-breeze"
	| "sage-garden"
	| "vercel"
	| "modern-minimal"
	| "violet-bloom"
	| "t3-chat"
	| "bubblegum"
	| "amethyst-haze"
	| "doom-64"
	| "catppuccin"
	| "graphite"
	| "tangerine"
	| "quantum-rose"
	| "bold-tech"
	| "supabase"
	| "neo-brutalism"
	| "solar-dusk"
	| "cyberpunk"
	| "pastel-dreams"
	| "caffeine"
	| "retro-arcade"
	| "midnight-bloom"
	| "candyland"
	| "northern-lights"
	| "vintage-paper"
	| "sunset-horizon"
	| "claude"
	| "darkmatter"
	| "mono"
	| "soft-pop";

/**
 * Uno de serie o uno de tweakcn instalado (`tw-<id>`). Los instalados se
 * añaden al final de `THEMES` al arrancar y al instalar o quitar uno
 * (`lib/temas-instalados.ts`): así el selector y el `ThemeProvider` los tratan
 * como a los demás.
 */
export type ThemeName = TemaDeSerie | `tw-${string}`;

export const THEMES: { name: ThemeName; label: string; swatch: string }[] = [
	// El primero a propósito: es el tema neutro de shadcn, la referencia contra
	// la que se compara cuando algo se ve raro en un tema de color.
	{ name: "base", label: "Base", swatch: "oklch(0.2050 0 0)" },
	{ name: "claymorphism", label: "Claymorphism", swatch: "oklch(0.5854 0.2041 277.1173)" },
	{ name: "starry-night", label: "Starry Night", swatch: "oklch(0.4815 0.1178 263.3758)" },
	{ name: "twitter", label: "Twitter", swatch: "oklch(0.6723 0.1606 244.9955)" },
	{ name: "perpetuity", label: "Perpetuity", swatch: "oklch(0.5624 0.0947 203.2755)" },
	{ name: "amber-minimal", label: "Amber Minimal", swatch: "oklch(0.7686 0.1647 70.0804)" },
	{ name: "clean-slate", label: "Clean Slate", swatch: "oklch(0.5854 0.2041 277.1173)" },
	{ name: "cosmic-night", label: "Cosmic Night", swatch: "oklch(0.5417 0.1790 288.0332)" },
	{ name: "elegant-luxury", label: "Elegant Luxury", swatch: "oklch(0.4650 0.1470 24.9381)" },
	{ name: "mocha-mousse", label: "Mocha Mousse", swatch: "oklch(0.6083 0.0623 44.3588)" },
	{ name: "kodama-grove", label: "Kodama Grove", swatch: "oklch(0.6657 0.1050 118.9078)" },
	{ name: "nature", label: "Nature", swatch: "oklch(0.5234 0.1347 144.1672)" },
	{ name: "notebook", label: "Notebook", swatch: "oklch(0.4891 0 0)" },
	{ name: "ocean-breeze", label: "Ocean Breeze", swatch: "oklch(0.7227 0.1920 149.5793)" },
	{ name: "sage-garden", label: "Sage Garden", swatch: "oklch(0.6333 0.0309 154.9039)" },
	{ name: "vercel", label: "Vercel", swatch: "oklch(0 0 0)" },
	// Añadidos el 13/10/2026: el resto de presets oficiales de tweakcn
	// (utils/theme-presets.ts del repositorio jnsahaj/tweakcn).
	{ name: "modern-minimal", label: "Modern Minimal", swatch: "#3b82f6" },
	{ name: "violet-bloom", label: "Violet Bloom", swatch: "#7033ff" },
	{ name: "t3-chat", label: "T3 Chat", swatch: "#a84370" },
	{ name: "bubblegum", label: "Bubblegum", swatch: "#d04f99" },
	{ name: "amethyst-haze", label: "Amethyst Haze", swatch: "#8a79ab" },
	{ name: "doom-64", label: "Doom 64", swatch: "#b71c1c" },
	{ name: "catppuccin", label: "Catppuccin", swatch: "#8839ef" },
	{ name: "graphite", label: "Graphite", swatch: "#606060" },
	{ name: "tangerine", label: "Tangerine", swatch: "#e05d38" },
	{ name: "quantum-rose", label: "Quantum Rose", swatch: "#e6067a" },
	{ name: "bold-tech", label: "Bold Tech", swatch: "#8b5cf6" },
	{ name: "supabase", label: "Supabase", swatch: "#72e3ad" },
	{ name: "neo-brutalism", label: "Neo Brutalism", swatch: "#ff3333" },
	{ name: "solar-dusk", label: "Solar Dusk", swatch: "#B45309" },
	{ name: "cyberpunk", label: "Cyberpunk", swatch: "#ff00c8" },
	{ name: "pastel-dreams", label: "Pastel Dreams", swatch: "#a78bfa" },
	{ name: "caffeine", label: "Caffeine", swatch: "#644a40" },
	{ name: "retro-arcade", label: "Retro Arcade", swatch: "#d33682" },
	{ name: "midnight-bloom", label: "Midnight Bloom", swatch: "#6c5ce7" },
	{ name: "candyland", label: "Candyland", swatch: "#ffc0cb" },
	{ name: "northern-lights", label: "Northern Lights", swatch: "#34a85a" },
	{ name: "vintage-paper", label: "Vintage Paper", swatch: "#a67c52" },
	{ name: "sunset-horizon", label: "Sunset Horizon", swatch: "#ff7e5f" },
	{ name: "claude", label: "Claude", swatch: "#c96442" },
	{ name: "darkmatter", label: "Darkmatter", swatch: "#d87943" },
	{ name: "mono", label: "Mono", swatch: "#737373" },
	{ name: "soft-pop", label: "Soft Pop", swatch: "#4f46e5" },
];
