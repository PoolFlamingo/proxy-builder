import { paraTraducir } from "@/components/language-provider";
import type { Vista } from "@/stores/ui-store";
import type { PasoBase } from "./logica";

/**
 * Un paso del tutorial: la zona que se resalta (el elemento con ese
 * `data-tutorial`) y lo que se cuenta de ella. Sin `ancla`, o si la zona no
 * está en pantalla (Diseñar sin una carta elegida, o una zona que en móvil
 * está en otra pestaña), la tarjeta sale en el centro. Con `tactil` (ver
 * `PasoBase`) el paso cuenta lo mismo sin clic derecho, teclas ni ratón
 * encima cuando se toca con el dedo.
 */
export type Paso = PasoBase;

export const PASOS: Record<Vista, Paso[]> = {
	deck: [
		{
			titulo: paraTraducir("Te damos la bienvenida a Proxy Builder"),
			texto: paraTraducir(
				"Monta un deck de Magic con imágenes de Scryfall o tuyas, diseña tus propias proxies y sácalas en un PDF listo para imprimir. Este recorrido enseña qué hay en cada zona; puedes repetirlo cuando quieras con el botón de ayuda de arriba.",
			),
		},
		{
			ancla: "pestanas",
			titulo: paraTraducir("Cinco pantallas"),
			texto: paraTraducir(
				"Deck: la lista, las hojas y la exportación. Diseñar: el editor de cartas, para hacer tus propias proxies. Biblioteca: todos tus decks guardados. Buscar: consulta cualquier carta. Jugar: la mesa, para jugar un deck en solitario o con más gente.",
			),
		},
		{
			ancla: "nombre-deck",
			titulo: paraTraducir("El nombre del deck"),
			texto: paraTraducir(
				"Haz clic para cambiarlo. El deck se guarda solo mientras trabajas: a la derecha verás cuándo está guardado.",
			),
		},
		{
			ancla: "lista-cuadro",
			titulo: paraTraducir("La lista de cartas"),
			texto: paraTraducir(
				"Pega aquí la lista, de Moxfield, Arena, MTGO o en texto plano («4 Lightning Bolt», con su set y número si quieres). «Cargar lista» busca las cartas en Scryfall y reemplaza el deck; «Añadir» las pone al final. Arriba, «Sets» deja elegir cartas de un set entero y «Ejemplo» pone una lista para probar.",
			),
		},
		{
			ancla: "lista-opciones",
			titulo: paraTraducir("Importar y opciones"),
			texto: paraTraducir(
				"Añade las fichas solas o detéctalas después, importa un deck desde una URL de Archidekt, sube imágenes propias de cartas o un reverso, y elige el idioma de las cartas, qué impresión se usa y la calidad de la vista previa.",
			),
		},
		{
			ancla: "vista-barra",
			titulo: paraTraducir("La barra de la vista previa"),
			texto: paraTraducir(
				"Añade cartas o páginas, ordena el deck solo, oculta las cartas de doble cara, mira los reversos o la información de cada carta, abre las estadísticas (curva, maná, robo, precios y legalidad) y cambia el zoom (también con Ctrl + rueda).",
			),
			tactil: {
				texto: paraTraducir(
					"Añade cartas o páginas, ordena el deck solo, oculta las cartas de doble cara, mira los reversos o la información de cada carta, abre las estadísticas (curva, maná, robo, precios y legalidad) y cambia el zoom.",
				),
			},
		},
		{
			ancla: "vista-hojas",
			titulo: paraTraducir("Las hojas"),
			texto: paraTraducir(
				"Así saldrá el papel. Arrastra las cartas para cambiar su orden o suelta encima imágenes de tu equipo. Con clic derecho en una carta: cambiar la variante, editarla, vincularla, diseñarla, marcarla como opcional o banquillo, fijarla… Con Ctrl o Mayús eliges varias; Ctrl+Z deshace.",
			),
			tactil: {
				texto: paraTraducir(
					"Así saldrá el papel. Arrastra las cartas para cambiar su orden. Mantén pulsada una carta para ver todo lo que se le puede hacer: cambiar la variante, editarla, vincularla, diseñarla, marcarla como opcional o banquillo, fijarla…",
				),
			},
		},
		{
			ancla: "exportar-boton",
			titulo: paraTraducir("Exportar"),
			texto: paraTraducir(
				"«Exportar PDF» crea el fichero para imprimir. Los tres puntos copian la lista para Moxfield, Arena o MTGO, la guardan en .txt o guardan el deck con sus imágenes (.pbdeck) para llevarlo a otro equipo.",
			),
		},
		{
			ancla: "exportar-ajustes",
			titulo: paraTraducir("Cómo sale el papel"),
			texto: paraTraducir(
				"Papel y orientación, tamaño de carta (63 × 88, 63,5 × 88,9 o 65 × 90 mm, que llena justa una funda), separación y sangrado, guías de corte, doble cara, oscurecer bordes, alinear filas para el dúplex y presets de impresión. Todo se guarda con el deck y se deshace con Ctrl+Z.",
			),
			tactil: {
				texto: paraTraducir(
					"Papel y orientación, tamaño de carta (63 × 88, 63,5 × 88,9 o 65 × 90 mm, que llena justa una funda), separación y sangrado, guías de corte, doble cara, oscurecer bordes, alinear filas para el dúplex y presets de impresión. Todo se guarda con el deck.",
				),
			},
		},
		{
			ancla: "ajustes",
			titulo: paraTraducir("Ajustes"),
			texto: paraTraducir(
				"El idioma de la app, el tema y el aspecto, los nombres de las cartas en otros idiomas y las actualizaciones.",
			),
		},
		{
			ancla: "ayuda",
			titulo: paraTraducir("Para repasar"),
			texto: paraTraducir(
				"Este botón vuelve a abrir el tutorial de la pantalla en la que estés. Diseñar y Biblioteca tienen el suyo.",
			),
		},
	],
	disenar: [
		{
			titulo: paraTraducir("Diseñar: el editor de cartas"),
			texto: paraTraducir(
				"Aquí haces tus propias proxies: la app pinta cada carta con su marco, sus textos y tu ilustración, en el idioma que elijas, y el PDF la imprime a su medida.",
			),
		},
		{
			ancla: "disenar-lista",
			titulo: paraTraducir("Las cartas del deck"),
			texto: paraTraducir(
				"Elige la carta que quieres diseñar; cada una dice si está sin diseño, en borrador o lista. Arriba se busca por nombre, y con Alt+← y Alt+→ pasas a la anterior o a la siguiente.",
			),
			tactil: {
				texto: paraTraducir(
					"Elige la carta que quieres diseñar; cada una dice si está sin diseño, en borrador o lista. Arriba se busca por nombre.",
				),
			},
		},
		{
			ancla: "disenar-barra",
			titulo: paraTraducir("Herramientas del deck"),
			texto: paraTraducir(
				"«Renumerar» numera las cartas por el orden del deck, «Diseñar el deck» las hace todas de una vez, «Exportar imágenes» las guarda en PNG, «Colecciones» define un estilo común (marco, pie, ajustes) y «Letras, marcos y símbolos» añade letras, packs de marcos y símbolos.",
			),
			tactil: {
				texto: paraTraducir(
					"Todo esto está en el menú «⋯» de la barra de arriba: «Renumerar» numera las cartas por el orden del deck, «Diseñar el deck» las hace todas de una vez, «Exportar imágenes» las guarda en PNG, «Colecciones» define un estilo común (marco, pie, ajustes) y «Letras, marcos y símbolos» añade letras, packs de marcos y símbolos.",
				),
			},
		},
		{
			ancla: "disenar-vista",
			titulo: paraTraducir("La vista previa"),
			texto: paraTraducir(
				"La carta tal como se imprimirá. Elige el idioma del diseño, pasa de una parte o cara a otra (las partidas, las Secuelas y las de doble cara tienen dos, cada una con su ilustración) y enciende el margen de seguridad. Arrastra la ilustración para moverla y usa la rueda para acercarla.",
			),
			tactil: {
				texto: paraTraducir(
					"La carta tal como se imprimirá. Elige el idioma del diseño, pasa de una parte o cara a otra (las partidas, las Secuelas y las de doble cara tienen dos, cada una con su ilustración) y enciende el margen de seguridad. Arrastra la ilustración para moverla.",
				),
			},
		},
		{
			ancla: "disenar-incidencias",
			titulo: paraTraducir("Avisos"),
			texto: paraTraducir(
				"Lo que conviene revisar: textos que no caben, símbolos que faltan, cartas sin texto oficial en ese idioma… Haz clic para verlos; algunos se arreglan con un botón.",
			),
		},
		{
			ancla: "disenar-formulario",
			titulo: paraTraducir("Lo que se puede cambiar"),
			texto: paraTraducir(
				"Carta: estilo y colección. Textos: nombre, tipo, reglas y ambientación en cada idioma. Arte: sube, pega, baja de Scryfall o recorta la ilustración de cada parte y encuádrala. Marco: pack, color y caja de reglas. Pie: número, set y artista. Ajustes finos: altura y tamaño de cada texto, radios y más.",
			),
		},
		{
			ancla: "disenar-pie",
			titulo: paraTraducir("Guardar"),
			texto: paraTraducir(
				"Marca la carta como lista para imprimir, crea una variante con otro diseño, guarda o guarda y pasa a la siguiente. Los cambios no llegan al deck hasta que guardas.",
			),
		},
	],
	buscar: [
		{
			titulo: paraTraducir("Buscar cartas"),
			texto: paraTraducir(
				"Consulta cualquier carta de Magic sin tocar tu deck: su texto, legalidad, precio e impresiones.",
			),
		},
		{
			ancla: "buscar-campo",
			titulo: paraTraducir("Qué buscar"),
			texto: paraTraducir(
				"Escribe un nombre, en cualquier idioma, o usa la sintaxis de Scryfall (t:dragon c:r).",
			),
		},
		{
			ancla: "buscar-filtros",
			titulo: paraTraducir("Filtros"),
			texto: paraTraducir(
				"Afina por colores, tipo, rareza, formato, coste, set o idioma. Los resultados cambian solos.",
			),
		},
		{
			ancla: "buscar-resultados",
			titulo: paraTraducir("Los resultados"),
			texto: paraTraducir(
				"Abre una carta para ver su ficha y sus otras impresiones, y añádela al deck abierto si quieres.",
			),
		},
	],
	biblioteca: [
		{
			titulo: paraTraducir("La biblioteca"),
			texto: paraTraducir(
				"Todos tus decks, que se guardan solos mientras trabajas. Haz clic en uno para abrirlo.",
			),
		},
		{
			ancla: "biblioteca-barra",
			titulo: paraTraducir("Buscar y ordenar"),
			texto: paraTraducir(
				"Busca por nombre, carta o nota (Ctrl+F); filtra por colores, formato, idioma, favoritos o cartas diseñadas; ordena, agrupa y cambia entre cuadrícula y lista.",
			),
			tactil: {
				texto: paraTraducir(
					"Busca por nombre, carta o nota; filtra por colores, formato, idioma, favoritos o cartas diseñadas; ordena, agrupa y cambia entre cuadrícula y lista.",
				),
			},
		},
		{
			ancla: "biblioteca-acciones",
			titulo: paraTraducir("Nuevo, importar y espacio"),
			texto: paraTraducir(
				"Crea un deck vacío, importa un .pbdeck (un deck con sus imágenes) o libera el espacio de las imágenes que ya no usa ningún deck.",
			),
		},
		{
			ancla: "biblioteca-lateral",
			titulo: paraTraducir("Etiquetas y favoritos"),
			texto: paraTraducir(
				"Ordena tus decks con etiquetas propias (formato, idioma, estado…) y con favoritos. Ctrl + clic para elegir varias etiquetas a la vez.",
			),
			tactil: {
				texto: paraTraducir(
					"Ordena tus decks con etiquetas propias (formato, idioma, estado…) y con favoritos.",
				),
			},
		},
		{
			ancla: "biblioteca-decks",
			titulo: paraTraducir("Tus decks"),
			texto: paraTraducir(
				"Cada deck enseña sus colores, cartas, precio e idiomas. Con clic derecho o los tres puntos: abrir, renombrar, duplicar, etiquetas, idioma, propiedades (notas y portada), exportar .pbdeck o eliminar. Con Ctrl o Mayús eliges varios y actúas sobre todos.",
			),
			tactil: {
				texto: paraTraducir(
					"Cada deck enseña sus colores, cartas, precio e idiomas. Mantén pulsado uno para elegirlo y toca otros para sumar; en sus tres puntos: abrir, renombrar, duplicar, etiquetas, idioma, propiedades (notas y portada), exportar .pbdeck o eliminar.",
				),
			},
		},
	],
	mesa: [
		{
			titulo: paraTraducir("La mesa de juego"),
			texto: paraTraducir(
				"Juega con tus decks como en una mesa de verdad: sin reglas programadas, solo las cartas, la vida, los contadores y los dados. Tú decides qué pasa.",
			),
		},
		{
			ancla: "mesa-entrada",
			titulo: paraTraducir("Elige deck y formato"),
			texto: paraTraducir(
				"Elige un deck de la biblioteca, el formato, las vidas y las ayudas (robar y enderezar al empezar el turno). Cada deck puede tener su tapete. Si dejaste una partida a medias, aquí la sigues.",
			),
		},
		{
			ancla: "mesa-solitario",
			titulo: paraTraducir("Solitario"),
			texto: paraTraducir(
				"Prepara las imágenes (la primera vez tarda un poco con las cartas diseñadas) y reparte la mano inicial.",
			),
		},
		{
			ancla: "mesa-red",
			titulo: paraTraducir("Jugar en red"),
			texto: paraTraducir(
				"Crea una partida y pasa el código a tus amigos, o pega el suyo para jugar o mirar. Las cartas propias y los tapetes de cada uno llegan solos.",
			),
		},
		{
			ancla: "mesa-barra",
			titulo: paraTraducir("La barra de la mesa"),
			texto: paraTraducir(
				"El turno y su reloj, pausa, deshacer y rehacer, dados y monedas, fichas, el registro, el tapete, los atajos y terminar o reiniciar la partida.",
			),
			tactil: {
				texto: paraTraducir(
					"El turno y su reloj, pausa, deshacer y rehacer, dados y monedas, fichas, el registro, el tapete y terminar o reiniciar la partida.",
				),
			},
		},
		{
			ancla: "mesa-campo",
			titulo: paraTraducir("El campo"),
			texto: paraTraducir(
				"Arrastra las cartas a donde quieras; doble clic las gira. Con clic derecho en una carta, todo lo que se le puede hacer; en el campo vacío, enderezar todo, organizar o crear fichas. Arrastra en el vacío para elegir varias.",
			),
			tactil: {
				texto: paraTraducir(
					"Arrastra las cartas a donde quieras. Mantén pulsada una carta para ver todo lo que se le puede hacer; en el campo vacío, enderezar todo, organizar o crear fichas.",
				),
			},
		},
		{
			ancla: "mesa-mano",
			titulo: paraTraducir("La mano"),
			texto: paraTraducir(
				"Arrastra una carta al campo para jugarla, o a otra zona. Pasa el ratón por cualquier carta para verla en grande.",
			),
			tactil: {
				texto: paraTraducir(
					"Arrastra una carta al campo para jugarla, o a otra zona. Toca una carta para verla en grande.",
				),
			},
		},
		{
			ancla: "mesa-biblioteca",
			titulo: paraTraducir("La biblioteca"),
			texto: paraTraducir(
				"Un clic roba una carta. Con clic derecho: moler, escrutar, vigilar, buscar, revelar y lanzar, barajar… Con el ratón encima, del 1 al 9 roba esas cartas.",
			),
			tactil: {
				texto: paraTraducir(
					"Un toque roba una carta. Mantén pulsada la biblioteca: moler, escrutar, vigilar, buscar, revelar y lanzar, barajar…",
				),
			},
		},
		{
			ancla: "mesa-vida",
			titulo: paraTraducir("Tu vida"),
			texto: paraTraducir(
				"− y + (o la rueda) cambian la vida; los clics seguidos se juntan. Con clic derecho: daño, veneno, daño de comandante, contadores y designaciones.",
			),
			tactil: {
				texto: paraTraducir(
					"− y + cambian la vida; los toques seguidos se juntan. Mantén pulsada la vida: daño, veneno, daño de comandante, contadores y designaciones.",
				),
			},
		},
		{
			ancla: "mesa-tablero",
			titulo: paraTraducir("Atajos"),
			texto: paraTraducir(
				"Casi todo tiene su tecla (T gira, G al cementerio, H a la mano…). Pulsa ? para ver la lista. Ctrl+Z deshace, sin límite en solitario.",
			),
			tactil: {
				titulo: paraTraducir("Los menús"),
				texto: paraTraducir(
					"Mantén pulsada una carta, una zona o el campo vacío para abrir su menú: ahí está todo lo que en un teclado tiene su tecla. Deshacer y rehacer están en la barra de arriba.",
				),
			},
		},
	],
};
