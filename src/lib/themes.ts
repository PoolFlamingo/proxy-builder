// Generado en su día por `scripts/build-themes.mjs` (que vive en MacUp, no en
// este repo). Aquí se edita a mano cuando hace falta, con cuidado de que el
// orden y los nombres casen con `src/themes.css`.
export type Mode = "light" | "dark" | "system";

export type ThemeName =
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
	| "vercel";

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
];
