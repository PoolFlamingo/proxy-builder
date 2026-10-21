import type { Diccionario } from "../tipos";

// Vincular cartas propias con Scryfall, zoom con la rueda y escala de la interfaz.
export const VINCULAR: Diccionario = {
	Comparar: "Сравнить",
	"Sin carta de Scryfall elegida.": "Карта Scryfall не выбрана.",
	"Tu carta": "Ваша карта",
	Scryfall: "Scryfall",
	"Ver en grande": "Крупно",
	"Todo el deck ({n})": "Вся колода ({n})",
	"Solo las seleccionadas ({n})": "Только выбранные ({n})",
	"Vincular con una carta": "Привязать к карте",
	"Ctrl + rueda del ratón para acercar o alejar": "Ctrl + колесо мыши — увеличить или уменьшить",
	"Zoom de la vista previa": "Масштаб предпросмотра",
	"Cambiar el vínculo…": "Изменить привязку…",
	"Vincular con una carta…": "Привязать к карте…",
	Reducir: "Уменьшить",
	Ampliar: "Увеличить",
	"Datos y precios de Scryfall. Las cartas propias cuentan si están vinculadas a su carta de Scryfall.":
		"Данные и цены из Scryfall. Свои карты учитываются, если они привязаны к карте Scryfall.",
	"Vinculada con {carta}": "Привязана к {carta}",
	"Di qué carta real es esta imagen propia: la imagen no cambia, pero la carta cuenta en las estadísticas y en la info. Se aplica a todas las copias con la misma imagen.":
		"Укажите, какой настоящей карте соответствует это изображение: оно не изменится, но карта будет учитываться в статистике и сведениях. Применяется ко всем копиям с тем же изображением.",
	"Detectar con OCR": "Распознать (OCR)",
	"La primera vez se descargan los modelos del OCR (unos 12 MB). Después funciona sin red.":
		"В первый раз скачиваются модели OCR (около 12 МБ). Потом всё работает без сети.",
	"Leído en la carta:": "Прочитано на карте:",
	"Nombre de la carta (en cualquier idioma)…": "Название карты (на любом языке)…",
	"Busca por nombre o pulsa «Detectar con OCR».":
		"Ищите по названию или нажмите «Распознать (OCR)».",
	"Quitar el vínculo": "Убрать привязку",
	"Vincular con {carta}": "Привязать к {carta}",
	Vincular: "Привязать",
	"Imagen propia vinculada a su carta de Scryfall":
		"Своё изображение, привязанное к карте Scryfall",
	"Busca en Scryfall la carta real de cada imagen propia (por el nombre del fichero o leyendo la carta: el título, el nombre original del pie o cualquier línea) y te enseña lo que ha encontrado para que lo revises antes de vincular. Con varias cartas seleccionadas, de partida solo esas.":
		"Ищет в Scryfall настоящую карту для каждого своего изображения (по имени файла или читая карту: название, оригинальное название в нижней строке или любую строку) и показывает найденное, чтобы вы проверили его перед привязкой. Если выбрано несколько карт, сначала только их.",
	"Vincular las seleccionadas…": "Привязать выбранные…",
	"quedan ~{n} s": "осталось ~{n} с",
	"quedan ~{n} min": "осталось ~{n} мин",
	"{n} cartas propias vinculadas": {
		one: "Привязана {n} своя карта",
		few: "Привязаны {n} свои карты",
		many: "Привязано {n} своих карт",
		other: "Привязано {n} своей карты",
	},
	"Vincular en bloque": "Массовая привязка",
	"{n} imágenes propias distintas entre las cartas seleccionadas.": {
		one: "{n} уникальное своё изображение среди выбранных карт.",
		few: "{n} разных своих изображения среди выбранных карт.",
		many: "{n} разных своих изображений среди выбранных карт.",
		other: "{n} разного своего изображения среди выбранных карт.",
	},
	"{n} imágenes propias distintas en el deck.": {
		one: "{n} уникальное своё изображение в колоде.",
		few: "{n} разных своих изображения в колоде.",
		many: "{n} разных своих изображений в колоде.",
		other: "{n} разного своего изображения в колоде.",
	},
	"¿Qué cartas quieres leer?": "Какие карты прочитать?",
	"Solo las que faltan": "Только недостающие",
	"{n} sin vincular": "Без привязки: {n}",
	"Todas otra vez": "Все заново",
	"{n} imágenes, también las ya vinculadas": {
		one: "{n} изображение, включая уже привязанные",
		few: "{n} изображения, включая уже привязанные",
		many: "{n} изображений, включая уже привязанные",
		other: "{n} изображения, включая уже привязанные",
	},
	"Leer la carta entera con OCR (más fiable, unos 2 s por carta)":
		"Читать всю карту через OCR (надёжнее, около 2 с на карту)",
	"Sin OCR solo se usa el nombre del fichero. La primera vez, el OCR descarga sus modelos (unos 12 MB).":
		"Без OCR используется только имя файла. В первый раз OCR скачивает свои модели (около 12 МБ).",
	"Sin OCR solo se usa el nombre del fichero: va rápido, pero acierta menos.":
		"Без OCR используется только имя файла: быстро, но менее точно.",
	"Leyendo {h} de {n}": "Чтение: {h} из {n}",
	"Vinculando {h} de {n}": "Привязка: {h} из {n}",
	"Exactas: {n}": "Точные: {n}",
	"Probables: {n}": "Вероятные: {n}",
	"Sin propuesta: {n}": "Без совпадения: {n}",
	"Sin marcar": "Не отмечены",
	"Empezar ({n})": "Начать ({n})",
	"Parar y revisar lo leído": "Остановить и проверить прочитанное",
	"Vincular {n}": "Привязать ({n})",
	Parar: "Остановить",
	"No vincular": "Не привязывать",
	"Vincular esta": "Привязать эту",
	"Carta de Scryfall": "Карта Scryfall",
	Exacta: "Точная",
	Probable: "Вероятная",
	"Sin propuesta": "Без совпадения",
};
