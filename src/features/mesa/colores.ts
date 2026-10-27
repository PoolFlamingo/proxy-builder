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

/**
 * Los contadores de fuerza y resistencia que existen en cartas reales (además
 * de +1/+1 y −1/−1, que van en los habituales). `ptDe` suma cualquier `+X/+Y`,
 * esté o no aquí: este es solo el catálogo del diálogo.
 */
export const CONTADORES_PT = [
	"+1/+0",
	"+0/+1",
	"+1/+2",
	"+0/+2",
	"+2/+2",
	"-1/-0",
	"-0/-1",
	"-2/-1",
	"-0/-2",
	"-2/-2",
] as const;

/**
 * El resto de tipos de contador de cartas reales (sacados del texto de reglas
 * de Scryfall, sin los de jugador ni los de habilidad), por su nombre en las
 * reglas en inglés, como las palabras clave. Es un catálogo para elegir: el
 * diálogo deja escribir cualquier otro.
 */
export const CONTADORES_OTROS: string[] =
	"acorn aegis age aim arrow arrowhead awakening bait blaze blessing blight blood bloodline bloodstain book bore bounty brain bribery brick burden cage carrion cell chorus coin collection component conqueror contested corpse corruption credit croak crystal cube currency death delay depletion descent despair devotion discovery divinity doom dread dream duty echo egg elixir ember enlightened eon eyeball fade fate feather feeding fellowship fetch filibuster film finality fire flame flood foreshadow fungus fury fuse gem ghostform glyph gold growth harmony hatching hatchling healing hit hone hoofprint hope hour hourglass hunger ice impostor incarnation incubation infection influence ingenuity ingredient intel intervention invasion invitation isolation javelin judgment ki kick knowledge landmark level loot luck magnet manifestation mannequin matrix memory midway mine mining mire music muster necrodermis nest net night omen ore page pain palliation paralyzation petal petrification phylactery phyresis pin plague plan plot point polyp possession pressure prey pupa quest rally refine rejection reprieve rev revival ribbon ritual rope rust saurian scream shell shred silver skewer sleep sleight slime slumber soot soul spite spore stash storage story strife study supply suspect takeover task theft tide tower training trap treasure unity unlock valor velocity verse vitality void vortex vow voyage wage winch wind wish wreck".split(
		" ",
	);

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
	"Decayed",
	"Exalted",
	"Shadow",
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
