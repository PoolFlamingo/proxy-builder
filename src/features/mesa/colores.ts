import { paraTraducir } from "@/components/language-provider";

/** Los 16 colores de resaltar y de las bolitas (`Bolita.color`, `Carta.resaltada`). */
export const COLORES = [
	"#ef4444",
	"#f97316",
	"#f59e0b",
	"#eab308",
	"#84cc16",
	"#22c55e",
	"#10b981",
	"#14b8a6",
	"#06b6d4",
	"#3b82f6",
	"#6366f1",
	"#8b5cf6",
	"#a855f7",
	"#ec4899",
	"#f5f5f4",
	"#1c1917",
] as const;

/** Los 7 colores de las etiquetas. */
export const COLORES_ETIQUETA = [
	"#ef4444",
	"#f59e0b",
	"#22c55e",
	"#3b82f6",
	"#8b5cf6",
	"#ec4899",
	"#78716c",
] as const;

/** Los colores de los jugadores (el primero, el de la app). */
export const COLORES_JUGADOR = [
	"#e11d48",
	"#2563eb",
	"#16a34a",
	"#ca8a04",
	"#9333ea",
	"#0891b2",
	"#ea580c",
	"#db2777",
	"#4f46e5",
	"#65a30d",
] as const;

/** Los colores de los equipos (hasta 5: con 10 jugadores, parejas). */
export const COLORES_EQUIPO = ["#f43f5e", "#3b82f6", "#22c55e", "#f59e0b", "#a855f7"] as const;

/** El equipo que se enfrenta al archienemigo: gris, para que no se confunda con ningún jugador. */
export const COLOR_CONTRA_ARCHIENEMIGO = "#94a3b8";

/** Los contadores de carta más usados, con su nombre (el tipo es el inglés de las reglas). */
export const CONTADORES_CARTA: { tipo: string; nombre: string }[] = [
	{ tipo: "+1/+1", nombre: "+1/+1" },
	{ tipo: "-1/-1", nombre: "-1/-1" },
	{ tipo: "loyalty", nombre: paraTraducir("Lealtad") },
	{ tipo: "lore", nombre: paraTraducir("Saga (lore)") },
	{ tipo: "charge", nombre: paraTraducir("Carga") },
	{ tipo: "time", nombre: paraTraducir("Tiempo") },
	{ tipo: "shield", nombre: paraTraducir("Escudo") },
	{ tipo: "stun", nombre: paraTraducir("Aturdimiento") },
	{ tipo: "oil", nombre: paraTraducir("Aceite") },
	{ tipo: "defense", nombre: paraTraducir("Defensa") },
];

/** Las palabras clave que se pueden poner como contador. */
export const PALABRAS_CLAVE = [
	"Flying",
	"First strike",
	"Double strike",
	"Deathtouch",
	"Hexproof",
	"Haste",
	"Indestructible",
	"Lifelink",
	"Menace",
	"Reach",
	"Trample",
	"Vigilance",
	"Ward",
	"Shroud",
] as const;

/** Los contadores de jugador («trackers»), con su nombre. */
export const CONTADORES_JUGADOR: { tipo: string; nombre: string }[] = [
	{ tipo: "energy", nombre: paraTraducir("Energía") },
	{ tipo: "experience", nombre: paraTraducir("Experiencia") },
	{ tipo: "treasure", nombre: paraTraducir("Tesoros") },
	{ tipo: "storm", nombre: paraTraducir("Tormenta") },
	{ tipo: "rad", nombre: paraTraducir("Rad") },
	{ tipo: "speed", nombre: paraTraducir("Velocidad") },
	{ tipo: "tax", nombre: paraTraducir("Impuesto") },
	{ tipo: "mana-W", nombre: paraTraducir("Maná blanco") },
	{ tipo: "mana-U", nombre: paraTraducir("Maná azul") },
	{ tipo: "mana-B", nombre: paraTraducir("Maná negro") },
	{ tipo: "mana-R", nombre: paraTraducir("Maná rojo") },
	{ tipo: "mana-G", nombre: paraTraducir("Maná verde") },
	{ tipo: "mana-C", nombre: paraTraducir("Maná incoloro") },
];
