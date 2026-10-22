# Packs de marcos

Un pack de marcos dice cómo se pinta una carta diseñada: dónde va cada cosa,
con qué colores y qué capas se dibujan encima de la ilustración. El de la
casa (`builtin`) va dentro de la app, en
`src-tauri/crates/proxy-core/assets/marcos/builtin/`, y está escrito en este
mismo formato. Un pack hecho por cualquiera funciona igual.

Se importa desde **Diseñar → Letras, marcos y símbolos → Packs de marcos**,
como carpeta o como `.zip`. La app lo copia a `app_data_dir/marcos/<id>/` y
lo valida al cargarlo: si algo está mal, dice qué (falta un fichero, un hueco,
un color mal escrito…). Los packs importados nunca van al repositorio.

## Ficheros

```
mi-pack/
├── pack.json
├── despejado/titulo.svg
├── despejado/reglas.svg
└── …
```

Las capas son SVG (con variables, ver abajo) o PNG (marcos de imagen, uno
por color, ver «Capas PNG»). Un pack puede mezclar las dos.

## Medidas

Todo se mide en **milímetros** sobre la carta de 63 × 88. El motor escala al
tamaño de la carta y a los píxeles de salida (la vista previa, el PDF a 300,
600 o 900 PPP y los PNG salen del mismo render). Un estilo o un layout puede
traer otra medida en `tamano` (`[ancho, alto]`): la batalla es `[88, 63]` y
el plano `[127, 88.9]`.

## `pack.json`

```json
{
  "id": "mi-pack",
  "nombre": "Mi pack",
  "version": 1,
  "paletas": { "W": { … }, "U": { … }, … },
  "letras": { "titulo": { … }, … },
  "estilos": { "borderless": { … }, "extended": { … }, … },
  "layouts": { "saga": { … }, "batalla": { … }, … }
}
```

- **`id`**: letras, números, `-` y `_`. No puede ser `builtin`. Importar otro
  pack con el mismo `id` lo sustituye.
- **`version`**: 1, 2 si usa algo de «Lo que añade la versión 2» o 3 si usa
  el trozo o la máscara de una capa PNG («Lo que añade la versión 3»). Una app
  anterior rechaza el pack en vez de pintarlo mal.

### Paletas

Una por color, con los nombres `W`, `U`, `B`, `R`, `G`, `gold`,
`artifact`, `land` y `colorless` (todas obligatorias). Cada una:

| Campo    | Para qué |
|----------|----------|
| `claro`, `medio`, `oscuro` | Brillo, cuerpo y sombra del marco (degradado). |
| `tinte`  | La caja de reglas translúcida (despejado). |
| `caja`, `tinta` | La caja de reglas clara (clásico, extendido) y su texto. |

Colores `#rgb`, `#rrggbb` o `#rrggbbaa`. El motor elige la paleta solo: un
color, la suya; dos, un degradado entre las dos; tres o más, `gold`; sin
color, `artifact`, `land` o `colorless`. El usuario puede forzar otra.

### Letras

Una por papel: `titulo`, `tipo`, `reglas`, `fr` (fuerza/resistencia y
números de las insignias), `pie` y `cinta`.

```json
"titulo": { "familia": "Spectral", "peso": 700, "mayuscula": 0.66 }
```

`mayuscula` es la altura de la mayúscula entre el cuerpo de la letra: el
texto se mide por la altura de sus mayúsculas, que es lo que se ve. La
familia tiene que estar cargada (las de la casa son Spectral y Montserrat; las
demás se importan en **Letras**). Una colección puede pedir otra letra para el
título y las reglas.

Una letra no tiene por qué traer todos los idiomas: lo que le falte del latín
(«ș», «ł»…) o del cirílico sale con Spectral o Montserrat, que lo tienen
todo, en vez de con cuadrados. El japonés, el coreano y el chino salen con
Noto Sans JP, KR, SC o TC, que la app baja la primera vez que se elige uno de
esos idiomas de diseño.

### Estilos

Los estilos que la app conoce son `borderless` (despejado), `extended`,
`classic` y `fullArt` (paisaje). Cada uno:

```json
"borderless": {
  "arte": { "x": 0, "y": 0, "w": 63, "h": 83 },
  "caja": { "fondo": 83, "altoMin": 12, "altoMax": 27.4, "barraTipo": 6,
            "padArriba": 0.95, "padAbajo": 0.75 },
  "cajaPorDefecto": "tinte",
  "capas": [ { "svg": "despejado/titulo.svg" },
             { "svg": "despejado/corona.svg", "si": { "corona": true } } ],
  "huecos": { "titulo": { … }, "tipo": { … }, "reglas": { … }, "pie": { … } },
  "layouts": { "ficha": { … }, "alto": { … } },
  "radio": 3
}
```

- **`arte`**: la ventana de la ilustración. La imagen la cubre, con el
  encuadre del usuario (zoom, desplazamiento y giro).
- **`caja`** (opcional): la caja de reglas que crece. Su borde de abajo queda
  fijo en `fondo` y el alto va de `altoMin` a `altoMax`; la barra de tipo va
  encima. Si el texto no cabe ni al máximo, la letra baja de 0,1 pt en 0,1 pt
  hasta 6 pt y, si aun así no cabe, se avisa. Sin `caja`, el hueco `reglas` es
  fijo y el texto se centra en vertical. La caja mide lo que pide el texto
  (con poco texto, o ninguno, queda baja y la ilustración llega más abajo);
  el usuario puede fijarla siempre en `altoMax` con el ajuste fino «Altura
  según el texto» (`rulesBoxDynamic: false`).
- **`cajaPorDefecto`**: `tinte` (translúcida del color de la carta) o `clara`.
- **`capas`**: los SVG que se pintan encima del arte, en orden. Con `si`,
  solo cuando se cumple: `corona` (legendaria en despejado o forzada), `fr`
  (lleva fuerza/resistencia), `cinta` (el nombre original va en la cinta),
  `emblema` (hay emblema).
- **`huecos`**: dónde va cada texto. Obligatorios: `titulo`, `tipo`,
  `reglas` y `pie`. Además: `coste`, `fr`, `cinta`, `emblema`, `simbolo`
  (paisaje), `mano` y `vida` (Vanguard).
- **`radio`**: el de las esquinas en pantalla (para imprimir salen
  cuadradas: el corte las redondea).
- **`coronaLegendaria`** (opcional): `true` para que las legendarias lleven
  corona sin pedirla. Sin decirlo, solo en el despejado.
- **`difuminadoArte`** (opcional, mm): funde con negro esos milímetros de
  abajo del hueco del arte, para que la ilustración no acabe en seco (el
  extendido de la casa lleva 12, con 7 de `extensionDifuminado`: lo que sigue la ilustración por debajo de su hueco, perdiéndose). Es un degradado encima del arte y debajo de
  las capas, suave al empezar y al acabar, y llega a negro del todo 1,5 mm
  antes del final del hueco (el marco suele tapar ese filo). Los ajustes
  finos de la carta o la colección lo cambian.
- La capa de la `cinta` va antes que la del marco cuando el marco tapa la
  barra del título: así queda por debajo de ella.
- **`ventanaArte`** (opcional): `{ x, y, w, h, radio, soloAbajo }` en mm. Lo
  que queda fuera se pinta de negro, con las esquinas a `radio` (o al de los
  ajustes finos de la carta o la colección). Va encima del arte y debajo de
  las capas. `soloAbajo` redondea solo las de abajo. Si la curva viene
  dibujada en una capa de imagen (los marcos Original), `repintar` lleva las
  zonas (`[x, y, ancho, alto]` en mm) donde, con un radio en los ajustes
  finos, se vuelve a pintar la ilustración o el negro encima de esa capa;
  `repintarTras` dice tras cuántas capas (para no tapar las de encima).

Los ajustes finos de una colección o una carta mueven en vertical los huecos
`titulo`, `coste`, `tipo`, `emblema`, `fr` y `pie`, y el texto de `reglas`;
también hacen más opaca la caja de reglas y cambian los radios.

Si a un pack le falta un estilo, ese estilo sale del pack de la casa (con las
paletas del pack). Lo mismo con las fichas: un estilo sin layout `ficha`
usa el de la casa.

### Capas PNG

En vez de `svg`, una capa puede llevar `png`: una imagen que se estira a la
carta entera o a la zona `en` (en mm). `{color}` en la ruta se cambia por la
letra del color del marco:

```json
{ "png": "m15/m15Frame{color}.png" },
{ "png": "m15/m15PT{color}.png", "en": { "x": 47.7, "y": 77.9, "w": 11.8, "h": 6.5 },
  "colores": { "land": "A" }, "si": { "fr": true } }
```

Las letras de fábrica son las de Card Conjurer: `W U B R G`, `M` (oro),
`A` (artefacto), `L` (tierra) y `C` (incoloro). `colores` en la raíz del
pack o en una capa cambia la de alguna paleta. Tiene que haber un PNG para
cada paleta (el pack no carga si falta uno). Las cartas de dos colores y las
de color propio usan el de oro.

Los PNG se decodifican al usarlos y se guardan un rato ya a la medida, así
que repintar con el mismo marco (al arrastrar el arte) no cuesta.

### Los marcos «Original»

El pack `original` (`crates/proxy-core/assets/marcos/original/`) son los
marcos M15 de Card Conjurer, con sus coronas y cajas de F/R por color, y los
tres estilos con el mismo mármol y pergamino:

- **Clásico**: el marco normal (`m15/regular/m15Frame{color}.png`).
- **Extendido**: el extendido (`m15/new/extended/{color}.png`, la ilustración
  hasta los bordes) con la barra del tipo de mármol del clásico encima (la
  suya es negra translúcida), recortada con la máscara `m15MaskType.png`, y
  la corona flotante (no hay bordes laterales bajo el título).
- **Despejado**: el marco despejado genérico de Card Conjurer, liso y
  translúcido, con las barras del título y del tipo del clásico (máscaras
  `m15MaskTitle.png` y `m15MaskType.png`), el pergamino de su caja de reglas
  (un trozo con `desde`, en su mismo sitio: el hueco entre los bordes de la
  caja es igual en los dos marcos) y su caja de F/R. El texto va oscuro. El pergamino
  solo sale con la caja automática o clara (`"si": {"caja": ["auto", "light"]}`):
  con la oscura o la de tinte queda la caja translúcida del marco, con el
  tinte encima, y se sigue viendo la ilustración. El borde de la caja es
  siempre el del marco.

Los tres estilos del pack `original` llevan `caja` con `fijaDeSerie`: la caja
mide `altoMax` de serie y solo se adapta a la altura del texto, como en el
despejado de la casa, si el usuario activa «Altura según el texto». Como la caja
viene pintada en el PNG con un alto fijo, el pack la rehace en tres trozos con
`desde` y `en`: el de arriba (con la barra del tipo) y el de abajo se copian
tal cual, y el de en medio se estira entre ellos con expresiones sobre `yTipo`
(`"y": "yTipo+6.9"`, `"h": "73.1 - yTipo"`). Las esquinas laterales del
despejado van en tiras aparte para que no se estiren. La capa `cajaReglas` y
`refuerzo-caja.svg` (que usa `{{yReglas}}` y `{{altoReglas}}`) siguen la misma
altura. Si quieres hacer lo mismo con otro marco de imagen, parte la caja
en esos tres trozos y revisa `ventanaArte.repintarTras` (cuántas capas se
pintan antes de repintar el arte) para que el arte no tape la caja.

En el repositorio solo va su `pack.json`, tres SVG propios e
`imagenes.json` (la lista de PNG con la huella SHA-256 de cada uno y el
commit fijo del que salen). La app los descarga la primera vez que se eligen
(`diseno/original.rs`), comprueba cada huella y los deja en
`app_data_dir/marcos/original/`. No se distribuyen con la app. Si una versión
nueva pide imágenes que aún no están, el pack carga sin los estilos que las
usan (caen a los de la casa) y la app las baja sola al arrancar.

### Huecos

```json
"titulo": { "x": 4.55, "y": 4.3, "w": 52.55, "h": 4.75,
            "rol": "titulo", "mayuscula": 2.15, "color": "#ffffff",
            "sombra": true, "alinear": "inicio" }
```

| Campo | Qué es |
|-------|--------|
| `x`, `y`, `w`, `h` | El rectángulo, en mm. Admiten expresiones (ver abajo). |
| `rol` | El papel de la letra. |
| `mayuscula` | Altura de la mayúscula en mm. |
| `color` | `#rrggbb[aa]` o una variable (`{{tintaCaja}}`). |
| `alinear` | `inicio`, `centro` o `fin`. |
| `sombra` | Sombra suave detrás del texto. |
| `versalitas` | Versalitas (los títulos de las fichas). |
| `tabs`, `interlinea` | Solo el pie: tabuladores en mm y separación de líneas. |

El título y la línea de tipo **se estrechan** a lo ancho hasta el límite de la
colección (del 55 % al 75 %, 65 % de fábrica) antes de bajar la letra.

### Variables

Las capas SVG llevan `{{variables}}` que el motor rellena antes de pintar:

| Variable | Qué es |
|----------|--------|
| `{{defs}}` | Los degradados: `url(#marco)`, `#brillo`, `#panel`, `#panelfr`, `#crema`, `#lateral`, `#vetas`, `#bisel`, `#barra`. Va dentro de `<defs>`. |
| `{{claro}}`, `{{medio}}`, `{{oscuro}}`, `{{tinte}}`, `{{caja}}` | Los colores de la paleta (o la mezcla de las dos). |
| `{{fondoCaja}}`, `{{opacidadCaja}}`, `{{tintaCaja}}` | La caja de reglas y el color de su texto. |
| `{{refuerzoCaja}}` | De 0 a 1, cuánto más opaca se ha pedido la caja (para los marcos con la caja en la imagen: un rectángulo negro debajo con esa opacidad). |
| `{{tintaCrema}}`, `{{tintaFr}}` | Texto sobre crema y sobre la caja de F/R. |
| `{{yReglas}}`, `{{altoReglas}}`, `{{yTipo}}`, `{{yTipo04}}` | La geometría de la caja que crece. |
| `{{extensionArte}}` | Cuántos mm sigue la ilustración por debajo de su hueco (la extensión del difuminado de los ajustes finos; 0 de fábrica). El negro de debajo del arte tiene que empezar ese tanto más abajo, si no, la tapa. |
| `{{subeCaja}}` | Cuántos mm más alta se ha pedido la caja de reglas en los ajustes finos (negativo, más baja; 0 de fábrica). |
| `{{= 56.25 - subeCaja}}` | Una cuenta (sumas y restas) con las variables que son números. Así un marco fijo mueve su barra de tipo y el borde de arriba de su caja. |

Los huecos pueden usar `yReglas`, `altoReglas`, `yTipo` y `subeCaja` en
expresiones de sumas y restas: `"y": "yTipo+0.4"`, `"h": "52.5 - subeCaja"`.
En los marcos fijos de la casa (extendido y clásico), el arte, la línea de
tipo, el emblema y las reglas van con `subeCaja`; en la caja que crece
(despejado), lo pedido se suma a su altura. Un pack que no use `subeCaja` no
cambia con ese ajuste.

El ajuste fino «Altura según el texto» (`rulesBoxDynamic`, en la carta o en la
colección) vale para todos los marcos: sin tocar (`None`) cada marco hace lo
suyo (se adapta en el despejado de la casa y es fija en los marcos SVG fijos
de la casa y en el pack `original`, por `fijaDeSerie`); en `true` los marcos
fijos con `subeCaja` también acortan su caja cuando el texto es corto (hasta
10 mm, calculado solo), y en `false` la caja se queda siempre en `altoMax`, con el texto centrado en vertical. En
el marco Balatro no cambia.

### Layouts de un estilo

Dentro de un estilo, `layouts` cambia partes de él para ciertas cartas. Lo
que no se nombra se queda como en el estilo.

- **`ficha`**: fichas, emblemas y fichas de dos caras (título centrado en
  versalitas…).
- **`alto`** (opcional): caja de reglas más alta para los cuerpos con
  secciones (planeswalker, clase, caso, leveler, Estación). Puede cambiar
  `arte`, `caja`, `capas` y `huecos`.

### Layouts del pack

`layouts` en la raíz del pack trae diseños **completos** (con su `arte`,
`capas`, `huecos` y, si hace falta, `tamano`) que valen para cualquier estilo.
Los usa la Fase 5:

| Layout | Para qué | Medida |
|--------|----------|--------|
| `saga` | Capítulos a la izquierda, arte a la derecha. | 63 × 88 |
| `mazmorra` | Banda de arte y salas unidas por flechas. | 63 × 88 |
| `secuelaArriba`, `secuelaAbajo` | Las dos mitades de una Secuela (la de abajo se pega girada). | 63 × 51, 33 × 63 |
| `fusionar` | La franja común de Fusionar. | 88 × 6 |
| `flip` | La mitad de arriba de una carta flip (la otra es la misma girada 180°). | 63 × 88 |
| `batalla` | La cara de delante de una batalla, apaisada. | 88 × 63 |
| `plano` | Plano de Planechase, apaisado. | 127 × 88,9 |
| `plan`, `vanguard` | Plan de Archenemy y Vanguard. | 88,9 × 127 |
| `pie` | Solo el pie, para las cartas compuestas. | 63 × 88 |

Si un pack no trae un layout que la carta necesita, se pinta con el estilo
normal y se avisa («el pack no tiene marco para…»). Las partidas, Fusionar y
Habitación no necesitan layout propio: son dos cartas normales giradas.

## Lo que añade la versión 2

Lo necesitó el pack Balatro (pixel art, siete plantillas distintas), pero vale
para cualquier pack.

### Estilos propios

Además de los cuatro de serie, `estilos` puede traer estilos con otro id
(`tarot`, `planeta`…). La app los ofrece en el selector de estilo de la carta
y de la colección mientras el pack esté elegido.

```json
"tarot": {
  "nombre": { "es": "Tarot", "en": "Tarot" },
  "base": "classic",
  "variantes": { "sinBarrido": { "es": "Sin barrido", "en": "No scanlines" } },
  "layoutSi": [ { "si": { "tipo": ["land"], "basica": false }, "layout": "noBasica" } ],
  "fondo": [ { "png": "tarot/fondo.png" } ],
  …lo mismo que un estilo de serie…
}
```

- **`nombre`**: lo que se lee en el selector, por idioma.
- **`base`**: de qué estilo de serie hereda lo que el motor decide por el
  estilo (esquinas, símbolo del paisaje…) y a cuál cae la carta si el pack no
  está. Sin decirlo, `classic`.
- **`variantes`**: opciones que elige el usuario (selector «Variante»).
  Encienden las capas con `si.variante` y, si el estilo tiene un layout con
  el mismo id, lo usan.
- **`layoutSi`**: layouts que se eligen solos por la carta; gana la primera
  regla que se cumpla.
- **`caja.altoComodo`** (en cualquier estilo con caja que crece): hasta
  dónde crece la caja antes de bajar la letra (hasta el 75 %). Así un texto
  largo no se come el arte. Si la carta pide un tamaño de reglas a mano (en
  los ajustes finos), manda ese.
- **`fondo`**: capas que se pintan **debajo** de la ilustración (el papel de
  una carta cuyo arte no la llena). Un layout también puede traer `fondo`.

Una carta con un estilo propio cuyo pack ya no está sale con su `base` y un
aviso.

### Condiciones (`si`)

Además de `corona`, `fr`, `cinta` y `emblema`:

| Campo | Se cumple si… |
|-------|---------------|
| `tipo` | la carta es de alguno de estos tipos (en inglés y minúsculas: `land`, `creature`, `token`…). |
| `basica`, `legendaria` | es (o no) tierra básica, legendaria. |
| `colores` | tiene entre `[min, max]` colores. |
| `simbolos` | su coste tiene entre `[min, max]` símbolos. |
| `variante` | el usuario eligió esa variante. |
| `caja` | (versión 3) la caja de reglas elegida en el diseño es una de estas: `auto`, `dark`, `light`, `tint`. |

### Capas

| Campo | Qué es |
|-------|--------|
| `escalado` | `suave` (de fábrica) o `pixel`: al ampliar cada píxel se repite, sin emborronar. `escalado` en la raíz del pack lo pone para todas. |
| `mezcla` | `normal`, `oscurecer` o `multiplicar`. |
| `opacidad` | De 0 a 1. |
| `tramos` | `[arriba, abajo]` en mm: esos trozos de arriba y de abajo se pintan sin deformar y solo se estira lo de en medio (una caja con esquinas que crece). |
| `cajaReglas` | `true` y una zona `en`, sin `png` ni `svg`: ahí se pinta el tinte de la caja de reglas cuando el diseño pide una caja oscura, clara o de otro tinte (para los marcos de imagen, que traen la caja pintada). Con ella, el texto de las reglas toma el color que se lee encima. Va en su sitio de la lista de capas (encima del marco y debajo de la caja de F/R, por ejemplo). |

En los marcos de imagen, un **degradado** elegido a mano pinta la capa de
cada color fundiendo la segunda de izquierda a derecha (entre el 35 % y el
65 % del ancho), y un **color propio** tiñe la capa del color neutro
(`colorless`) conservando sus luces y sombras. Solo las capas con `{color}`
en la ruta. Las cartas de dos colores en automático siguen en oro, como las
de verdad.

La zona `en` de una capa PNG admite las mismas expresiones que los huecos
(`"y": "yTipo+1.6"`, `"h": "80-yTipo"`): así la barra del tipo y la caja de
reglas se mueven con la caja que crece (`caja` del estilo). Con una caja fija
esas variables no existen y la capa no se pinta. El hueco `arte` también las
admite, para que la ilustración llegue hasta la barra del tipo.

En la ruta de un PNG, `{color1}` y `{color2}` son el primer y el segundo color
de la carta en orden WUBRG (sin color, la letra de incoloro; con uno, los dos
el mismo): los cuadros de color junto al título.

### Huecos

| Campo | Qué es |
|-------|--------|
| `mayusculas` | Todo el texto en mayúsculas. |
| `alinearV` | En una caja fija, dónde va el texto en vertical: `inicio`, `centro` (de fábrica) o `fin`. |
| `contenido` | Solo en el hueco `etiqueta`: `rareza` (de fábrica), `supertipo` o `subtipoTierra`. |

`alinear: "centro"` en el hueco `coste` centra los símbolos.

### Textos por idioma

`nombre` y `variantes` de los estilos, el `texto` de las `etiquetas` y la
`plantillaPie` son mapas **código de idioma → texto**. Los códigos son los de
la app (los de Scryfall): `es`, `en`, `fr`, `de`, `it`, `pt` (portugués de
Brasil), `ro`, `pl`, `nl`, `ru`, `uk`, `ja`, `ko`, `zhs` (chino simplificado)
y `zht` (chino tradicional). Se puede traer los que se quiera:

- **`nombre`** y **`variantes`** se leen en el idioma de la **interfaz**; sin
  ese idioma, en inglés; sin inglés, el id.
- **`etiquetas`** y **`plantillaPie`** se pintan en el idioma de **diseño** de
  la carta; sin ese idioma, en inglés.
- Una clave que no es de ningún idioma de la app se ignora.
- `{lang}` en la plantilla del pie es el código que llevan las cartas
  impresas en ese idioma (`FR`, `DE`, `JP`, `KR`, `CS`, `CT`…; el español sigue
  saliendo como `ES`).

### Raíz del pack

```json
"simbolos": { "W": "symbols/W.png", "T": "symbols/T.png" },
"etiquetas": { "rare": { "texto": { "es": "Rara", "en": "Rare" },
                         "fondo": "labels/rare.png", "color": "#ffffff" } },
"insignias": { "fondo": "#3a3a3a", "borde": "#000000", "texto": "#ffffff",
               "pngLealtad": "insignias/lealtad.png" },
"coloresTexto": { "resalte": "#4aa0ff", "habilidad": "#ff9b2e" },
"sustituir": { "—": "-", "•": "·" },
"separacionCoste": 0.1,
"plantillaPie": { "es": "{set} • {lang} {num} {rarity}\t{original} {brush} {artist}" }
```

- **`simbolos`**: símbolos propios (código sin llaves → SVG o PNG del pack)
  que tapan a los de Scryfall en el coste y en las reglas. Los que no trae
  siguen siendo los de Scryfall.
- **`etiquetas`**: la píldora del hueco `etiqueta`, por valor (`common`,
  `uncommon`, `rare`, `mythic`, `legendary`, `mountain`…). `fondo` es un PNG
  del pack o un color.
- **`insignias`**: colores de las insignias de las secciones y, si se quiere,
  una imagen para la de lealtad.
- **`coloresTexto`**: en las reglas, el color de lo marcado con `[[así]]` y
  el de las palabras de habilidad. Sin él, las marcas se quitan y todo sale del
  color normal.
- **`sustituir`**: caracteres que su letra no tiene y por cuáles se cambian,
  solo al escribir con este pack.
- **`separacionCoste`**: hueco entre los símbolos del coste, en proporción a
  su lado (0,05 de fábrica).
- **`plantillaPie`**: el pie de fábrica de este pack por idioma (sin el del
  idioma, el inglés), con las claves de las colecciones. Solo cuando la
  colección no pone uno propio. Si el hueco `pie` no tiene `tabs`, las
  columnas (separadas por tabulador) van seguidas y, si la línea no cabe, se
  estrecha; con `tabs`, cada columna se estrecha hasta el tabulador
  siguiente. Lo que va tras `{>}` en una línea se pega a la derecha del
  hueco (y lo demás llega hasta ahí). Si la plantilla no pone `{universe}`,
  `{year}` ni `{credit}`, salen solos tras `{>}` en la última línea
  («Monster Hunter · © 2026 Capcom»).

## Lo que añade la versión 3

La condición `caja` (ver «Condiciones») y dos campos de las capas PNG para
sacar una pieza de otro marco:

| Campo | Qué es |
|-------|--------|
| `desde` | Una zona en mm (`x`, `y`, `w`, `h`; admite las mismas variables que `en`) medida como si la imagen cubriera la carta entera: solo se pinta ese trozo. Sin `en` va en el mismo sitio de la carta; con `en`, estirado a esa zona. |
| `mascara` | Un PNG del pack de la medida de la carta: la capa solo se ve donde la máscara es opaca (multiplica su transparencia). Con `desde`, se usa el mismo trozo de la máscara. |

```json
{ "png": "m15/regular/m15Frame{color}.png", "mascara": "m15/regular/m15MaskType.png" },
{ "png": "m15/regular/m15Frame{color}.png",
  "desde": { "x": 4.72, "y": 55.2, "w": 53.66, "h": 26.2 } }
```

Ninguno de los dos va con `tramos` ni en capas SVG. La máscara no lleva
`{color}`: es la misma para todos los colores. El degradado y el color propio
del marco (ver «Capas») se aplican igual al trozo.

## Packs para descargar

Los packs que no van con la app (el Balatro) viven en el repositorio
`PoolFlamingo/proxy-builder-packs`. La app trae su lista en
`src-tauri/assets/packs-descargables.json`: un commit fijo y la huella SHA-256
de cada fichero, como los marcos Original. Se descargan desde **Letras,
marcos y símbolos** o al elegir el pack en el selector de marco
(`diseno/descargables.rs`). Las imágenes extra (reversos, fichas y tierras ya
hechas) quedan en `app_data_dir/packs-extras/<id>/` y se añaden al deck como
cartas propias. Al bajar una versión nueva solo se descarga lo que ha cambiado
y se quita lo que su manifiesto ya no tiene. En desarrollo, `PB_PACKS_LOCAL=<carpeta>` lee los packs de
una copia local en lugar de bajarlos.

## Lo que el motor pone solo

- La ilustración, el coste (símbolos de Scryfall), el emblema, la corona y
  la caja de F/R.
- Las insignias de las secciones (escudos de lealtad, capítulos, niveles,
  prototipo) y las franjas, en los colores de la carta.
- El indicador de cara de las transformables (sol y luna) y de las MDFC
  (triángulos), con la pestaña de la otra cara.
- Las palabras fijas en el idioma de diseño: la de los niveles de un leveler
  («NIVEL», «STUFE», «等级»…) y el recordatorio de Fusionar.
- Los emoji de cualquier texto, con Noto Color Emoji (COLRv1, con sus
  degradados).

## Comprobar un pack

`cargo test -p proxy-core --test dorados --test layouts` pinta las cartas de
prueba con el pack de la casa. Para probar uno propio, impórtalo en la app y
mira la vista previa de cada estilo: los avisos de abajo dicen si falta algo.
