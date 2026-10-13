// Estadísticas completas del deck: pestañas, funciones, probabilidades, precios
// y legalidad.
export const ESTADISTICAS: Record<string, string> = {
	// Pestañas y cabecera.
	Resumen: "Overview",
	Maná: "Mana",
	Funciones: "Roles",
	Probabilidades: "Probabilities",
	Precios: "Prices",
	"Leyendo los datos de las cartas…": "Reading card data…",
	"Automático: {c}": "Automatic: {c}",
	"Commander (100)": "Commander (100)",
	"Construido (60)": "Constructed (60)",
	"Limitado (40)": "Limited (40)",

	// Resumen.
	"{n} sin tierras · {t} tokens": "{n} non-land · {t} tokens",
	"{n} sin tierras": "{n} non-land",
	"{p} % · recomendadas ~{r}": "{p} % · recommended ~{r}",
	"Recomendación de la fórmula de Frank Karsten para esta curva: {r} tierras (descuenta {b} cartas baratas de rampa o robo).":
		"Frank Karsten's formula for this curve recommends {r} lands (it subtracts {b} cheap ramp or draw cards).",
	"con tierras {c} · mediana {m}": "with lands {c} · median {m}",
	"Valor de maná": "Mana value",
	"suma de costes sin tierras": "sum of non-land costs",
	"todas con precio": "all priced",
	"Bracket (estimado)": "Bracket (estimate)",
	"{n} Game Changers": "{n} Game Changers",
	"Probabilidad de 2-4 tierras": "Chance of 2-4 lands",
	"en la mano inicial": "in the opening hand",
	"Sin datos de Scryfall (propias sin vincular): no cuentan":
		"No Scryfall data (unlinked custom cards): not counted",
	"Más de {n} copias": "More than {n} copies",
	"El mazo tiene {a} cartas y se esperan {e}.": "The deck has {a} cards and {e} are expected.",
	"Por tipo": "By type",
	"Permanentes y hechizos": "Permanents and spells",
	permanentes: "permanents",
	"instantáneos y conjuros": "instants and sorceries",
	Permanentes: "Permanents",
	"Instantáneos y conjuros": "Instants and sorceries",
	"{c}: {n} cartas, {p} símbolos de maná": "{c}: {n} cards, {p} mana symbols",
	"{n} con X en el coste": "{n} with X in the cost",
	"Tokens que usa el deck": "Tokens the deck uses",
	"Ninguno en el deck.": "None in the deck.",

	// Maná.
	"Símbolos de maná frente a fuentes": "Mana symbols vs sources",
	"Qué parte de los símbolos de color de los costes es de cada color, y qué parte de las tierras lo produce. Si un color pide mucho más de lo que dan las tierras, costará lanzarlo a tiempo.":
		"How much of the coloured symbols in the costs belongs to each colour, and how much of the lands produce it. If a colour asks for much more than the lands give, it will be hard to cast on time.",
	"Sin datos de maná.": "No mana data.",
	Costes: "Costs",
	"{n} símbolos · {p} %": "{n} symbols · {p} %",
	"{n} tierras · {p} %": "{n} lands · {p} %",
	"+ {n} fuentes que no son tierra": "+ {n} non-land sources",
	"Pocas fuentes para lo que piden los costes: {c}": "Few sources for what the costs need: {c}",
	"Cartas por número de colores": "Cards by number of colours",
	"Cartas de cada color": "Cards of each colour",
	Incoloras: "Colourless",
	Monocolor: "Mono-coloured",
	"2 colores": "2 colours",
	"3 colores": "3 colours",
	"4 colores": "4 colours",
	"5 colores": "5 colours",

	// Tipos.
	Legendarias: "Legendary",
	Básicas: "Basic",
	Nevadas: "Snow",
	Mundo: "World",
	"En curso": "Ongoing",
	"Fuerza media": "Average power",
	"Resistencia media": "Average toughness",
	"{n} criaturas de fuerza {f}": "{n} creatures with power {f}",
	Subtipos: "Subtypes",
	"Ninguno.": "None.",
	Habilidades: "Keywords",
	"Ninguna.": "None.",
	"{n} sets · de {a} a {b}": "{n} sets · from {a} to {b}",

	// Funciones.
	"Se deduce del texto de reglas de cada carta: es orientativo. Una carta puede tener varias funciones. El botón de la derecha selecciona esas cartas en el deck.":
		"Worked out from each card's rules text, so take it as a guide. A card can have several roles. The button on the right selects those cards in the deck.",
	"No se ha detectado ninguna.": "None detected.",
	"Cartas de cada función": "Cards for each role",
	"Sin cartas: {f}": "No cards: {f}",
	"Seleccionar estas cartas en el deck": "Select these cards in the deck",
	"{n} cartas seleccionadas: {f}": "{n} cards selected: {f}",
	Rampa: "Ramp",
	"Da maná o pone tierras en juego (piedras, criaturas de maná, Tesoros…).":
		"Makes mana or puts lands onto the battlefield (rocks, mana creatures, Treasures…).",
	Robo: "Card draw",
	"Roba cartas o exilia de la biblioteca para jugarlas.":
		"Draws cards or exiles from the library to play them.",
	Eliminación: "Removal",
	"Destruye, exilia, hace daño, devuelve a la mano o hace sacrificar un objetivo.":
		"Destroys, exiles, deals damage to, bounces or makes someone sacrifice a target.",
	Barridos: "Board wipes",
	"Elimina todas las criaturas (o permanentes) a la vez.":
		"Removes every creature (or permanent) at once.",
	Contrahechizos: "Counterspells",
	"Contrarresta hechizos o habilidades.": "Counters spells or abilities.",
	Tutores: "Tutors",
	"Busca en la biblioteca algo que no es una tierra.":
		"Searches the library for something that isn't a land.",
	Protección: "Protection",
	"Da antimaleficio, indestructible, protección, custodia o hace fasear.":
		"Gives hexproof, indestructible, protection or ward, or phases out.",
	Recursión: "Recursion",
	"Recupera cartas del cementerio o deja jugarlas desde allí.":
		"Gets cards back from the graveyard or lets you play them from there.",
	Fichas: "Tokens",
	"Crea fichas.": "Creates tokens.",
	Potenciadores: "Anthems",
	"Da +X/+X a las criaturas que controlas.": "Gives +X/+X to creatures you control.",
	"Ganar vidas": "Lifegain",
	"Ganas vidas o tiene vínculo vital.": "You gain life, or it has lifelink.",
	Moler: "Mill",
	"Pone cartas de la biblioteca en el cementerio.": "Puts cards from a library into a graveyard.",
	"Turnos extra": "Extra turns",
	"Da un turno adicional.": "Gives an extra turn.",
	"Destrucción de tierras": "Land destruction",
	"Destruye o hace sacrificar tierras en masa, o no las deja enderezar.":
		"Destroys or sacrifices lands en masse, or stops them untapping.",

	// Probabilidades.
	"{f} ({n} cartas)": "{f} ({n} cards)",
	"Mano inicial": "Opening hand",
	"Turno {n}": "Turn {n}",
	"El deck está vacío.": "The deck is empty.",
	"Calculadora de robo": "Draw calculator",
	"Probabilidad de haber robado al menos una copia antes de un turno, con un mazo de {n} cartas y mano de 7 (sin mulligan).":
		"Chance of having drawn at least one copy by a given turn, with a {n}-card deck and a 7-card hand (no mulligan).",
	"Empiezo yo": "On the play",
	"Empieza el rival": "On the draw",
	"{v} cartas vistas": "{v} cards seen",
	Mano: "Hand",
	"Tierras en la mano inicial": "Lands in the opening hand",
	"{n} de media": "{n} on average",
	"{k} tierras: {p}": "{k} lands: {p}",
	"Manos de 2 a 4 tierras: {p} · con 0-1 o 5+: {q}":
		"Hands with 2 to 4 lands: {p} · with 0-1 or 5+: {q}",
	"Llegar a las tierras de cada turno": "Hitting land drops",
	"Probabilidad de tener al menos N tierras en el turno N.":
		"Chance of having at least N lands on turn N.",
	Turno: "Turn",

	// Precios.
	Total: "Total",
	"Media por carta": "Average per card",
	"La más cara": "Most expensive",
	"Sin precio": "No price",
	"copias sin precio en {m}": "copies with no {m} price",
	"Precios de Scryfall (Cardmarket en euros, TCGplayer en dólares) de la impresión elegida, sin foil. Se actualizan una vez al día.":
		"Scryfall prices (Cardmarket in euros, TCGplayer in dollars) for the chosen printing, non-foil. Updated once a day.",
	"Copias por precio (euros)": "Copies by price (euros)",
	"Cartas, de la más cara a la más barata": "Cards, from most to least expensive",
	"Ver menos": "Show fewer",
	"Ver todas ({n})": "Show all ({n})",
	Carta: "Card",
	Unidad: "Each",
	"Seleccionar en el deck": "Select in the deck",

	// Legalidad.
	"Legalidad por formato": "Legality by format",
	"Según la lista de prohibidas de Scryfall de cada carta. No mira el tamaño del mazo ni las copias (eso va abajo).":
		"Based on each card's Scryfall ban list. It doesn't check deck size or copies (that's below).",
	"Construcción como {c}": "Deck building as {c}",
	"Tamaño y copias correctos.": "Size and copies are fine.",
	"Bracket de Commander (estimado)": "Commander bracket (estimate)",
	"Es el mínimo que permiten las cartas: no detecta combos ni la intención del mazo, que también cuentan.":
		"It's the lowest the cards allow: it can't detect combos or the deck's intent, which also count.",
	"Game Changers": "Game Changers",
	"Destrucción masiva de tierras": "Mass land destruction",
	Núcleo: "Core",
	"Sin Game Changers, sin destrucción masiva de tierras y sin encadenar turnos extra. (El 1, Exhibición, depende del tema del mazo y no se puede deducir.)":
		"No Game Changers, no mass land destruction and no chained extra turns. (Bracket 1, Exhibition, depends on the deck's theme and can't be worked out.)",
	Mejorado: "Upgraded",
	"Hasta tres Game Changers, sin destrucción masiva de tierras ni combos de dos cartas tempranos.":
		"Up to three Game Changers, no mass land destruction and no early two-card combos.",
	Optimizado: "Optimized",
	"Sin restricciones: más de tres Game Changers o destrucción masiva de tierras.":
		"No restrictions: more than three Game Changers or mass land destruction.",
};
