import type { Diccionario } from "../tipos";

// Estadísticas completas del deck: pestañas, funciones, probabilidades, precios
// y legalidad.
export const ESTADISTICAS: Diccionario = {
	// Pestañas y cabecera.
	Resumen: "Обзор",
	Maná: "Мана",
	Funciones: "Роли",
	Probabilidades: "Вероятности",
	Precios: "Цены",
	"Leyendo los datos de las cartas…": "Чтение данных карт…",
	"Automático: {c}": "Автоматически: {c}",
	"Commander (100)": "Commander (100)",
	"Construido (60)": "Конструктед (60)",
	"Limitado (40)": "Лимитед (40)",

	// Resumen.
	"{n} sin tierras · {t} tokens": "Не земель: {n} · фишек: {t}",
	"{n} sin tierras": "Не земель: {n}",
	"{p} % · recomendadas ~{r}": "{p} % · рекомендуется ~{r}",
	"Recomendación de la fórmula de Frank Karsten para esta curva: {r} tierras (descuenta {b} cartas baratas de rampa o robo).":
		"Формула Фрэнка Карстена для этой кривой рекомендует земель: {r} (за вычетом дешёвых карт рампа и добора: {b}).",
	"con tierras {c} · mediana {m}": "с землями {c} · медиана {m}",
	"Valor de maná": "Общая мана-стоимость",
	"suma de costes sin tierras": "сумма стоимостей без земель",
	"todas con precio": "у всех есть цена",
	"Bracket (estimado)": "Брекет (оценка)",
	"{n} Game Changers": "Game Changers: {n}",
	"Probabilidad de 2-4 tierras": "Вероятность 2–4 земель",
	"en la mano inicial": "в стартовой руке",
	"Sin datos de Scryfall (propias sin vincular): no cuentan":
		"Нет данных Scryfall (непривязанные свои карты): не учитываются",
	"Más de {n} copias": {
		one: "Больше {n} копии",
		few: "Больше {n} копий",
		many: "Больше {n} копий",
		other: "Больше {n} копии",
	},
	"El mazo tiene {a} cartas y se esperan {e}.": "В колоде карт: {a}, а ожидается {e}.",
	"Por tipo": "По типу",
	"Permanentes y hechizos": "Перманенты и заклинания",
	permanentes: "перманенты",
	"instantáneos y conjuros": "мгновенные заклинания и волшебства",
	Permanentes: "Перманенты",
	"Instantáneos y conjuros": "Мгновенные заклинания и волшебства",
	"{c}: {n} cartas, {p} símbolos de maná": {
		one: "{c}: {n} карта, символов маны: {p}",
		few: "{c}: {n} карты, символов маны: {p}",
		many: "{c}: {n} карт, символов маны: {p}",
		other: "{c}: {n} карты, символов маны: {p}",
	},
	"{n} con X en el coste": "С X в стоимости: {n}",
	"Tokens que usa el deck": "Фишки колоды",
	"Ninguno en el deck.": "В колоде нет.",

	// Maná.
	"Símbolos de maná frente a fuentes": "Символы маны и источники",
	"Qué parte de los símbolos de color de los costes es de cada color, y qué parte de las tierras lo produce. Si un color pide mucho más de lo que dan las tierras, costará lanzarlo a tiempo.":
		"Какая доля цветных символов в стоимостях приходится на каждый цвет и какая доля земель его производит. Если цвет требует намного больше, чем дают земли, его будет трудно разыгрывать вовремя.",
	"Sin datos de maná.": "Нет данных о мане.",
	Costes: "Стоимости",
	"{n} símbolos · {p} %": {
		one: "{n} символ · {p} %",
		few: "{n} символа · {p} %",
		many: "{n} символов · {p} %",
		other: "{n} символа · {p} %",
	},
	"{n} tierras · {p} %": {
		one: "{n} земля · {p} %",
		few: "{n} земли · {p} %",
		many: "{n} земель · {p} %",
		other: "{n} земли · {p} %",
	},
	"+ {n} fuentes que no son tierra": {
		one: "+ {n} источник — не земля",
		few: "+ {n} источника — не земли",
		many: "+ {n} источников — не земли",
		other: "+ {n} источника — не земли",
	},
	"Pocas fuentes para lo que piden los costes: {c}": "Мало источников для таких стоимостей: {c}",
	"Cartas por número de colores": "Карты по числу цветов",
	"Cartas de cada color": "Карты каждого цвета",
	Incoloras: "Бесцветные",
	Monocolor: "Одноцветные",
	"2 colores": "2 цвета",
	"3 colores": "3 цвета",
	"4 colores": "4 цвета",
	"5 colores": "5 цветов",

	// Tipos.
	Legendarias: "Легендарные",
	Básicas: "Базовые",
	Nevadas: "Снежные",
	Mundo: "Мир",
	"En curso": "Длящиеся",
	"Fuerza media": "Средняя сила",
	"Resistencia media": "Средняя выносливость",
	"{n} criaturas de fuerza {f}": {
		one: "{n} существо с силой {f}",
		few: "{n} существа с силой {f}",
		many: "{n} существ с силой {f}",
		other: "{n} существа с силой {f}",
	},
	Subtipos: "Подтипы",
	"Ninguno.": "Нет.",
	Habilidades: "Ключевые слова",
	"Ninguna.": "Нет.",
	"{n} sets · de {a} a {b}": {
		one: "{n} выпуск · с {a} по {b}",
		few: "{n} выпуска · с {a} по {b}",
		many: "{n} выпусков · с {a} по {b}",
		other: "{n} выпуска · с {a} по {b}",
	},

	// Funciones.
	"Se deduce del texto de reglas de cada carta: es orientativo. Una carta puede tener varias funciones. El botón de la derecha selecciona esas cartas en el deck.":
		"Определяется по тексту правил каждой карты, поэтому это лишь ориентир. У карты может быть несколько ролей. Кнопка справа выделяет эти карты в колоде.",
	"No se ha detectado ninguna.": "Ничего не найдено.",
	"Cartas de cada función": "Карты по ролям",
	"Sin cartas: {f}": "Нет карт: {f}",
	"Seleccionar estas cartas en el deck": "Выделить эти карты в колоде",
	"{n} cartas seleccionadas: {f}": {
		one: "Выделена {n} карта: {f}",
		few: "Выделены {n} карты: {f}",
		many: "Выделено {n} карт: {f}",
		other: "Выделено {n} карты: {f}",
	},
	Rampa: "Рамп",
	"Da maná o pone tierras en juego (piedras, criaturas de maná, Tesoros…).":
		"Даёт ману или выкладывает земли на поле битвы (артефакты маны, существа маны, Сокровища…).",
	Robo: "Добор",
	"Roba cartas o exilia de la biblioteca para jugarlas.":
		"Берёт карты или изгоняет их из библиотеки, чтобы разыграть.",
	Eliminación: "Удаление",
	"Destruye, exilia, hace daño, devuelve a la mano o hace sacrificar un objetivo.":
		"Уничтожает, изгоняет, наносит повреждения, возвращает в руку или заставляет пожертвовать цель.",
	Barridos: "Массовое удаление",
	"Elimina todas las criaturas (o permanentes) a la vez.":
		"Убирает всех существ (или все перманенты) разом.",
	Contrahechizos: "Контрзаклинания",
	"Contrarresta hechizos o habilidades.": "Отменяет заклинания или способности.",
	Tutores: "Тьюторы",
	"Busca en la biblioteca algo que no es una tierra.": "Ищет в библиотеке что-то, кроме земли.",
	Protección: "Защита",
	"Da antimaleficio, indestructible, protección, custodia o hace fasear.":
		"Даёт Порчеустойчивость, Неразрушимость, Защиту или Опеку либо уводит из фазы.",
	Recursión: "Возврат",
	"Recupera cartas del cementerio o deja jugarlas desde allí.":
		"Возвращает карты с кладбища или позволяет разыгрывать их оттуда.",
	Fichas: "Фишки",
	"Crea fichas.": "Создаёт фишки.",
	Potenciadores: "Усилители",
	"Da +X/+X a las criaturas que controlas.": "Даёт +X/+X существам под вашим контролем.",
	"Ganar vidas": "Получение жизни",
	"Ganas vidas o tiene vínculo vital.": "Вы получаете жизни, или у карты есть Цепь жизни.",
	Moler: "Милл",
	"Pone cartas de la biblioteca en el cementerio.": "Кладёт карты из библиотеки на кладбище.",
	"Turnos extra": "Дополнительные ходы",
	"Da un turno adicional.": "Даёт дополнительный ход.",
	"Destrucción de tierras": "Уничтожение земель",
	"Destruye o hace sacrificar tierras en masa, o no las deja enderezar.":
		"Массово уничтожает земли, заставляет их пожертвовать или не даёт им разворачиваться.",

	// Probabilidades.
	"{f} ({n} cartas)": {
		one: "{f} ({n} карта)",
		few: "{f} ({n} карты)",
		many: "{f} ({n} карт)",
		other: "{f} ({n} карты)",
	},
	"Mano inicial": "Стартовая рука",
	"Turno {n}": "Ход {n}",
	"El deck está vacío.": "Колода пуста.",
	"Calculadora de robo": "Калькулятор добора",
	"Probabilidad de haber robado al menos una copia antes de un turno, con un mazo de {n} cartas y mano de 7 (sin mulligan).":
		{
			one: "Вероятность взять хотя бы одну копию к заданному ходу при колоде из {n} карты и руке из 7 (без муллигана).",
			few: "Вероятность взять хотя бы одну копию к заданному ходу при колоде из {n} карт и руке из 7 (без муллигана).",
			many: "Вероятность взять хотя бы одну копию к заданному ходу при колоде из {n} карт и руке из 7 (без муллигана).",
			other:
				"Вероятность взять хотя бы одну копию к заданному ходу при колоде из {n} карты и руке из 7 (без муллигана).",
		},
	"Empiezo yo": "Хожу первым",
	"Empieza el rival": "Первым ходит соперник",
	"{v} cartas vistas": "Просмотрено карт: {v}",
	Mano: "Рука",
	"Tierras en la mano inicial": "Земли в стартовой руке",
	"{n} de media": "В среднем {n}",
	"{k} tierras: {p}": "Земель: {k} — {p}",
	"Manos de 2 a 4 tierras: {p} · con 0-1 o 5+: {q}": "Руки с 2–4 землями: {p} · с 0–1 или 5+: {q}",
	"Llegar a las tierras de cada turno": "Земля на каждый ход",
	"Probabilidad de tener al menos N tierras en el turno N.":
		"Вероятность иметь хотя бы N земель на ходу N.",
	Turno: "Ход",

	// Precios.
	Total: "Итого",
	"Media por carta": "В среднем за карту",
	"La más cara": "Самая дорогая",
	"Sin precio": "Без цены",
	"copias sin precio en {m}": "копий без цены в {m}",
	"Precios de Scryfall (Cardmarket en euros, TCGplayer en dólares) de la impresión elegida, sin foil. Se actualizan una vez al día.":
		"Цены Scryfall (Cardmarket в евро, TCGplayer в долларах) для выбранного издания, без фойла. Обновляются раз в день.",
	"Copias por precio (euros)": "Копии по цене (евро)",
	"Cartas, de la más cara a la más barata": "Карты, от самой дорогой к самой дешёвой",
	"Ver menos": "Показать меньше",
	"Ver todas ({n})": "Показать все ({n})",
	Carta: "Карта",
	Unidad: "За шт.",
	"Seleccionar en el deck": "Выделить в колоде",

	// Legalidad.
	"Legalidad por formato": "Легальность по форматам",
	"Según la lista de prohibidas de Scryfall de cada carta. No mira el tamaño del mazo ni las copias (eso va abajo).":
		"По списку запрещённых карт в Scryfall. Размер колоды и число копий здесь не проверяются (это ниже).",
	"Construcción como {c}": "Сборка как {c}",
	"Tamaño y copias correctos.": "Размер и число копий в порядке.",
	"Bracket de Commander (estimado)": "Брекет Commander (оценка)",
	"Es el mínimo que permiten las cartas: no detecta combos ni la intención del mazo, que también cuentan.":
		"Это минимум, который допускают карты: комбо и замысел колоды, которые тоже важны, не учитываются.",
	"Game Changers": "Game Changers",
	"Destrucción masiva de tierras": "Массовое уничтожение земель",
	Núcleo: "Базовый",
	"Sin Game Changers, sin destrucción masiva de tierras y sin encadenar turnos extra. (El 1, Exhibición, depende del tema del mazo y no se puede deducir.)":
		"Без Game Changers, без массового уничтожения земель и без цепочек дополнительных ходов. (Брекет 1, «Показательный», зависит от темы колоды и не определяется автоматически.)",
	Mejorado: "Улучшенный",
	"Hasta tres Game Changers, sin destrucción masiva de tierras ni combos de dos cartas tempranos.":
		"До трёх Game Changers, без массового уничтожения земель и ранних комбо из двух карт.",
	Optimizado: "Оптимизированный",
	"Sin restricciones: más de tres Game Changers o destrucción masiva de tierras.":
		"Без ограничений: больше трёх Game Changers или массовое уничтожение земель.",
	"Aparte, por su marca (no cuentan aquí): {v} variantes de diseño, {o} opcionales y {b} en el banquillo.":
		"Отложены по отметке (здесь не учитываются): вариантов дизайна — {v}, необязательных — {o}, в сайдборде — {b}.",
};
