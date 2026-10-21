import { paraTraducir } from "@/components/language-provider";
import { esLang, localeDe } from "@/lib/idiomas";

/** Nombre y aspecto de cada color de maná (fijos: el color tiene significado). */
export const COLORES: Record<string, { nombre: string; chip: string; barra: string }> = {
	W: {
		nombre: paraTraducir("Blanco"),
		chip: "bg-amber-100 text-amber-900",
		barra: "bg-amber-200",
	},
	U: { nombre: paraTraducir("Azul"), chip: "bg-sky-500 text-white", barra: "bg-sky-500" },
	B: {
		nombre: paraTraducir("Negro"),
		chip: "bg-neutral-800 text-white",
		barra: "bg-neutral-600 dark:bg-neutral-400",
	},
	R: { nombre: paraTraducir("Rojo"), chip: "bg-red-600 text-white", barra: "bg-red-600" },
	G: { nombre: paraTraducir("Verde"), chip: "bg-green-600 text-white", barra: "bg-green-600" },
	C: { nombre: paraTraducir("Incoloro"), chip: "bg-stone-400 text-black", barra: "bg-stone-400" },
};

/** Los tipos principales, con el color con el que se apilan en la curva. */
export const TIPOS: Record<string, { nombre: string; clase: string }> = {
	Creature: { nombre: paraTraducir("Criaturas"), clase: "bg-emerald-500" },
	Planeswalker: { nombre: paraTraducir("Planeswalkers"), clase: "bg-amber-500" },
	Battle: { nombre: paraTraducir("Batallas"), clase: "bg-orange-600" },
	Instant: { nombre: paraTraducir("Instantáneos"), clase: "bg-sky-500" },
	Sorcery: { nombre: paraTraducir("Conjuros"), clase: "bg-rose-500" },
	Artifact: { nombre: paraTraducir("Artefactos"), clase: "bg-zinc-400" },
	Enchantment: { nombre: paraTraducir("Encantamientos"), clase: "bg-violet-500" },
	Land: { nombre: paraTraducir("Tierras"), clase: "bg-yellow-700" },
	Other: { nombre: paraTraducir("Otros"), clase: "bg-pink-400" },
};

export const SUPERTIPOS: Record<string, string> = {
	Legendary: paraTraducir("Legendarias"),
	Basic: paraTraducir("Básicas"),
	Snow: paraTraducir("Nevadas"),
	World: paraTraducir("Mundo"),
	Ongoing: paraTraducir("En curso"),
};

export const RAREZAS: Record<string, string> = {
	common: paraTraducir("Común"),
	uncommon: paraTraducir("Infrecuente"),
	rare: paraTraducir("Rara"),
	mythic: paraTraducir("Mítica"),
	special: paraTraducir("Especial"),
	bonus: paraTraducir("Bonus"),
};

/** Las funciones que detecta Rust por el texto de reglas. */
export const FUNCIONES: Record<string, { nombre: string; ayuda: string }> = {
	ramp: {
		nombre: paraTraducir("Rampa"),
		ayuda: paraTraducir("Da maná o pone tierras en juego (piedras, criaturas de maná, Tesoros…)."),
	},
	draw: {
		nombre: paraTraducir("Robo"),
		ayuda: paraTraducir("Roba cartas o exilia de la biblioteca para jugarlas."),
	},
	removal: {
		nombre: paraTraducir("Eliminación"),
		ayuda: paraTraducir(
			"Destruye, exilia, hace daño, devuelve a la mano o hace sacrificar un objetivo.",
		),
	},
	wipe: {
		nombre: paraTraducir("Barridos"),
		ayuda: paraTraducir("Elimina todas las criaturas (o permanentes) a la vez."),
	},
	counter: {
		nombre: paraTraducir("Contrahechizos"),
		ayuda: paraTraducir("Contrarresta hechizos o habilidades."),
	},
	tutor: {
		nombre: paraTraducir("Tutores"),
		ayuda: paraTraducir("Busca en la biblioteca algo que no es una tierra."),
	},
	protection: {
		nombre: paraTraducir("Protección"),
		ayuda: paraTraducir("Da antimaleficio, indestructible, protección, custodia o hace fasear."),
	},
	recursion: {
		nombre: paraTraducir("Recursión"),
		ayuda: paraTraducir("Recupera cartas del cementerio o deja jugarlas desde allí."),
	},
	tokens: { nombre: paraTraducir("Fichas"), ayuda: paraTraducir("Crea fichas.") },
	anthem: {
		nombre: paraTraducir("Potenciadores"),
		ayuda: paraTraducir("Da +X/+X a las criaturas que controlas."),
	},
	lifegain: {
		nombre: paraTraducir("Ganar vidas"),
		ayuda: paraTraducir("Ganas vidas o tiene vínculo vital."),
	},
	mill: {
		nombre: paraTraducir("Moler"),
		ayuda: paraTraducir("Pone cartas de la biblioteca en el cementerio."),
	},
	extraTurn: {
		nombre: paraTraducir("Turnos extra"),
		ayuda: paraTraducir("Da un turno adicional."),
	},
	landDenial: {
		nombre: paraTraducir("Destrucción de tierras"),
		ayuda: paraTraducir("Destruye o hace sacrificar tierras en masa, o no las deja enderezar."),
	},
};

export const CLASES_MAZO: Record<string, string> = {
	commander: paraTraducir("Commander (100)"),
	constructed: paraTraducir("Construido (60)"),
	limited: paraTraducir("Limitado (40)"),
};

/** Los brackets de Commander (los oficiales). */
export const BRACKETS: Record<number, { nombre: string; ayuda: string }> = {
	2: {
		nombre: paraTraducir("Núcleo"),
		ayuda: paraTraducir(
			"Sin Game Changers, sin destrucción masiva de tierras y sin encadenar turnos extra. (El 1, Exhibición, depende del tema del mazo y no se puede deducir.)",
		),
	},
	3: {
		nombre: paraTraducir("Mejorado"),
		ayuda: paraTraducir(
			"Hasta tres Game Changers, sin destrucción masiva de tierras ni combos de dos cartas tempranos.",
		),
	},
	4: {
		nombre: paraTraducir("Optimizado"),
		ayuda: paraTraducir(
			"Sin restricciones: más de tres Game Changers o destrucción masiva de tierras.",
		),
	},
};

export type Moneda = "eur" | "usd";

export function formatoMoneda(lang: string, moneda: Moneda) {
	return new Intl.NumberFormat(localeDe(esLang(lang) ? lang : "en"), {
		style: "currency",
		currency: moneda === "eur" ? "EUR" : "USD",
	});
}
