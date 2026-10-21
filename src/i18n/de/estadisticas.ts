import type { Diccionario } from "../tipos";

// Estadísticas completas del deck: pestañas, funciones, probabilidades, precios
// y legalidad.
export const ESTADISTICAS: Diccionario = {
	// Pestañas y cabecera.
	Resumen: "Übersicht",
	Maná: "Mana",
	Funciones: "Funktionen",
	Probabilidades: "Wahrscheinlichkeiten",
	Precios: "Preise",
	"Leyendo los datos de las cartas…": "Kartendaten werden gelesen…",
	"Automático: {c}": "Automatisch: {c}",
	"Commander (100)": "Commander (100)",
	"Construido (60)": "Constructed (60)",
	"Limitado (40)": "Limited (40)",

	// Resumen.
	"{n} sin tierras · {t} tokens": "{n} ohne Länder · {t} Spielsteine",
	"{n} sin tierras": "{n} ohne Länder",
	"{p} % · recomendadas ~{r}": "{p} % · empfohlen ~{r}",
	"Recomendación de la fórmula de Frank Karsten para esta curva: {r} tierras (descuenta {b} cartas baratas de rampa o robo).":
		"Die Formel von Frank Karsten empfiehlt für diese Kurve {r} Länder (abzüglich {b} günstiger Ramp- oder Zieh-Karten).",
	"con tierras {c} · mediana {m}": "mit Ländern {c} · Median {m}",
	"Valor de maná": "Manabetrag",
	"suma de costes sin tierras": "Summe der Kosten ohne Länder",
	"todas con precio": "alle mit Preis",
	"Bracket (estimado)": "Bracket (geschätzt)",
	"{n} Game Changers": "{n} Game Changer",
	"Probabilidad de 2-4 tierras": "Chance auf 2–4 Länder",
	"en la mano inicial": "auf der Starthand",
	"Sin datos de Scryfall (propias sin vincular): no cuentan":
		"Ohne Scryfall-Daten (nicht verknüpfte eigene Karten): zählen nicht",
	"Más de {n} copias": { one: "Mehr als {n} Kopie", other: "Mehr als {n} Kopien" },
	"El mazo tiene {a} cartas y se esperan {e}.": "Das Deck hat {a} Karten, erwartet werden {e}.",
	"Por tipo": "Nach Typ",
	"Permanentes y hechizos": "Bleibende Karten und Zaubersprüche",
	permanentes: "bleibende Karten",
	"instantáneos y conjuros": "Spontanzauber und Hexereien",
	Permanentes: "Bleibende Karten",
	"Instantáneos y conjuros": "Spontanzauber und Hexereien",
	"{c}: {n} cartas, {p} símbolos de maná": {
		one: "{c}: {n} Karte, {p} Manasymbole",
		other: "{c}: {n} Karten, {p} Manasymbole",
	},
	"{n} con X en el coste": "{n} mit X in den Kosten",
	"Tokens que usa el deck": "Spielsteine des Decks",
	"Ninguno en el deck.": "Keine im Deck.",

	// Maná.
	"Símbolos de maná frente a fuentes": "Manasymbole und Quellen",
	"Qué parte de los símbolos de color de los costes es de cada color, y qué parte de las tierras lo produce. Si un color pide mucho más de lo que dan las tierras, costará lanzarlo a tiempo.":
		"Welcher Anteil der farbigen Symbole in den Kosten auf jede Farbe fällt und welcher Anteil der Länder sie erzeugt. Verlangt eine Farbe viel mehr, als die Länder liefern, wird es schwer, rechtzeitig zu wirken.",
	"Sin datos de maná.": "Keine Manadaten.",
	Costes: "Kosten",
	"{n} símbolos · {p} %": { one: "{n} Symbol · {p} %", other: "{n} Symbole · {p} %" },
	"{n} tierras · {p} %": { one: "{n} Land · {p} %", other: "{n} Länder · {p} %" },
	"+ {n} fuentes que no son tierra": {
		one: "+ {n} Quelle, die kein Land ist",
		other: "+ {n} Quellen, die keine Länder sind",
	},
	"Pocas fuentes para lo que piden los costes: {c}": "Zu wenige Quellen für die Kosten: {c}",
	"Cartas por número de colores": "Karten nach Anzahl der Farben",
	"Cartas de cada color": "Karten je Farbe",
	Incoloras: "Farblos",
	Monocolor: "Einfarbig",
	"2 colores": "2 Farben",
	"3 colores": "3 Farben",
	"4 colores": "4 Farben",
	"5 colores": "5 Farben",

	// Tipos.
	Legendarias: "Legendär",
	Básicas: "Standard",
	Nevadas: "Verschneit",
	Mundo: "Welt",
	"En curso": "Fortlaufend",
	"Fuerza media": "Durchschnittliche Stärke",
	"Resistencia media": "Durchschnittliche Widerstandskraft",
	"{n} criaturas de fuerza {f}": {
		one: "{n} Kreatur mit Stärke {f}",
		other: "{n} Kreaturen mit Stärke {f}",
	},
	Subtipos: "Untertypen",
	"Ninguno.": "Keine.",
	Habilidades: "Schlüsselwörter",
	"Ninguna.": "Keine.",
	"{n} sets · de {a} a {b}": {
		one: "{n} Set · von {a} bis {b}",
		other: "{n} Sets · von {a} bis {b}",
	},

	// Funciones.
	"Se deduce del texto de reglas de cada carta: es orientativo. Una carta puede tener varias funciones. El botón de la derecha selecciona esas cartas en el deck.":
		"Abgeleitet aus dem Regeltext jeder Karte, also nur ein Richtwert. Eine Karte kann mehrere Funktionen haben. Die Schaltfläche rechts wählt diese Karten im Deck aus.",
	"No se ha detectado ninguna.": "Keine erkannt.",
	"Cartas de cada función": "Karten je Funktion",
	"Sin cartas: {f}": "Ohne Karten: {f}",
	"Seleccionar estas cartas en el deck": "Diese Karten im Deck auswählen",
	"{n} cartas seleccionadas: {f}": {
		one: "{n} Karte ausgewählt: {f}",
		other: "{n} Karten ausgewählt: {f}",
	},
	Rampa: "Ramp",
	"Da maná o pone tierras en juego (piedras, criaturas de maná, Tesoros…).":
		"Erzeugt Mana oder bringt Länder ins Spiel (Mana-Artefakte, Mana-Kreaturen, Schätze…).",
	Robo: "Kartenziehen",
	"Roba cartas o exilia de la biblioteca para jugarlas.":
		"Zieht Karten oder schickt sie aus der Bibliothek ins Exil, um sie zu spielen.",
	Eliminación: "Removal",
	"Destruye, exilia, hace daño, devuelve a la mano o hace sacrificar un objetivo.":
		"Zerstört, schickt ins Exil, fügt Schaden zu, bringt auf die Hand zurück oder lässt ein Ziel opfern.",
	Barridos: "Boardwipes",
	"Elimina todas las criaturas (o permanentes) a la vez.":
		"Entfernt alle Kreaturen (oder bleibenden Karten) auf einmal.",
	Contrahechizos: "Neutralisieren",
	"Contrarresta hechizos o habilidades.": "Neutralisiert Zaubersprüche oder Fähigkeiten.",
	Tutores: "Tutoren",
	"Busca en la biblioteca algo que no es una tierra.":
		"Durchsucht die Bibliothek nach etwas, das kein Land ist.",
	Protección: "Schutz",
	"Da antimaleficio, indestructible, protección, custodia o hace fasear.":
		"Verleiht Fluchsicherheit, Unzerstörbarkeit, Schutz oder Abwehr oder lässt herausphasen.",
	Recursión: "Rekursion",
	"Recupera cartas del cementerio o deja jugarlas desde allí.":
		"Holt Karten aus dem Friedhof zurück oder lässt sie von dort spielen.",
	Fichas: "Spielsteine",
	"Crea fichas.": "Erzeugt Spielsteine.",
	Potenciadores: "Verstärker",
	"Da +X/+X a las criaturas que controlas.": "Gibt Kreaturen, die du kontrollierst, +X/+X.",
	"Ganar vidas": "Lebenspunkte",
	"Ganas vidas o tiene vínculo vital.": "Du erhältst Lebenspunkte oder sie hat Lebensverknüpfung.",
	Moler: "Mahlen",
	"Pone cartas de la biblioteca en el cementerio.":
		"Legt Karten aus einer Bibliothek auf einen Friedhof.",
	"Turnos extra": "Zusatzzüge",
	"Da un turno adicional.": "Gibt einen zusätzlichen Zug.",
	"Destrucción de tierras": "Landzerstörung",
	"Destruye o hace sacrificar tierras en masa, o no las deja enderezar.":
		"Zerstört Länder massenhaft, lässt sie opfern oder verhindert, dass sie enttappt werden.",

	// Probabilidades.
	"{f} ({n} cartas)": { one: "{f} ({n} Karte)", other: "{f} ({n} Karten)" },
	"Mano inicial": "Starthand",
	"Turno {n}": "Zug {n}",
	"El deck está vacío.": "Das Deck ist leer.",
	"Calculadora de robo": "Ziehrechner",
	"Probabilidad de haber robado al menos una copia antes de un turno, con un mazo de {n} cartas y mano de 7 (sin mulligan).":
		"Chance, bis zu einem bestimmten Zug mindestens eine Kopie gezogen zu haben, bei einem Deck mit {n} Karten und einer Starthand von 7 (ohne Mulligan).",
	"Empiezo yo": "Ich beginne",
	"Empieza el rival": "Gegner beginnt",
	"{v} cartas vistas": "{v} Karten gesehen",
	Mano: "Hand",
	"Tierras en la mano inicial": "Länder auf der Starthand",
	"{n} de media": "{n} im Schnitt",
	"{k} tierras: {p}": "{k} Länder: {p}",
	"Manos de 2 a 4 tierras: {p} · con 0-1 o 5+: {q}":
		"Hände mit 2 bis 4 Ländern: {p} · mit 0–1 oder 5+: {q}",
	"Llegar a las tierras de cada turno": "Landdrops erreichen",
	"Probabilidad de tener al menos N tierras en el turno N.":
		"Chance, in Zug N mindestens N Länder zu haben.",
	Turno: "Zug",

	// Precios.
	Total: "Gesamt",
	"Media por carta": "Schnitt pro Karte",
	"La más cara": "Die teuerste",
	"Sin precio": "Ohne Preis",
	"copias sin precio en {m}": "Kopien ohne Preis bei {m}",
	"Precios de Scryfall (Cardmarket en euros, TCGplayer en dólares) de la impresión elegida, sin foil. Se actualizan una vez al día.":
		"Scryfall-Preise (Cardmarket in Euro, TCGplayer in Dollar) der gewählten Druckversion, nicht foil. Einmal täglich aktualisiert.",
	"Copias por precio (euros)": "Kopien nach Preis (Euro)",
	"Cartas, de la más cara a la más barata": "Karten, von der teuersten zur günstigsten",
	"Ver menos": "Weniger zeigen",
	"Ver todas ({n})": "Alle zeigen ({n})",
	Carta: "Karte",
	Unidad: "Stück",
	"Seleccionar en el deck": "Im Deck auswählen",

	// Legalidad.
	"Legalidad por formato": "Legalität nach Format",
	"Según la lista de prohibidas de Scryfall de cada carta. No mira el tamaño del mazo ni las copias (eso va abajo).":
		"Nach der Bannliste von Scryfall für jede Karte. Deckgröße und Kopien werden nicht geprüft (siehe unten).",
	"Construcción como {c}": "Deckbau als {c}",
	"Tamaño y copias correctos.": "Größe und Kopien sind in Ordnung.",
	"Bracket de Commander (estimado)": "Commander-Bracket (geschätzt)",
	"Es el mínimo que permiten las cartas: no detecta combos ni la intención del mazo, que también cuentan.":
		"Das ist das Minimum, das die Karten erlauben: Kombos und die Absicht des Decks, die ebenfalls zählen, werden nicht erkannt.",
	"Game Changers": "Game Changer",
	"Destrucción masiva de tierras": "Massenlandzerstörung",
	Núcleo: "Kern",
	"Sin Game Changers, sin destrucción masiva de tierras y sin encadenar turnos extra. (El 1, Exhibición, depende del tema del mazo y no se puede deducir.)":
		"Keine Game Changer, keine Massenlandzerstörung und keine aneinandergereihten Zusatzzüge. (Bracket 1, Exhibition, hängt vom Thema des Decks ab und lässt sich nicht ableiten.)",
	Mejorado: "Verbessert",
	"Hasta tres Game Changers, sin destrucción masiva de tierras ni combos de dos cartas tempranos.":
		"Bis zu drei Game Changer, keine Massenlandzerstörung und keine frühen Zwei-Karten-Kombos.",
	Optimizado: "Optimiert",
	"Sin restricciones: más de tres Game Changers o destrucción masiva de tierras.":
		"Keine Einschränkungen: mehr als drei Game Changer oder Massenlandzerstörung.",
	"Aparte, por su marca (no cuentan aquí): {v} variantes de diseño, {o} opcionales y {b} en el banquillo.":
		"Wegen ihrer Markierung beiseitegelegt (hier nicht gezählt): {v} Designvarianten, {o} optional und {b} im Sideboard.",
};
