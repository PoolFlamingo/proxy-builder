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
- **`version`**: 1.

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
  fijo y el texto se centra en vertical.
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
marcos M15 de Card Conjurer: normal (clásico) y despejado, con sus coronas y
cajas de F/R por color. En el repositorio solo va su `pack.json`, tres SVG
propios y `imagenes.json` (la lista de PNG con la huella SHA-256 de cada
uno y el commit fijo del que salen). La app los descarga la primera vez que
se eligen (`diseno/original.rs`), comprueba cada huella y los deja en
`app_data_dir/marcos/original/`. No se distribuyen con la app.

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

Los huecos pueden usar `yReglas`, `altoReglas` y `yTipo` en expresiones de
sumas y restas: `"y": "yTipo+0.4"`.

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

## Lo que el motor pone solo

- La ilustración, el coste (símbolos de Scryfall), el emblema, la corona y
  la caja de F/R.
- Las insignias de las secciones (escudos de lealtad, capítulos, niveles,
  prototipo) y las franjas, en los colores de la carta.
- El indicador de cara de las transformables (sol y luna) y de las MDFC
  (triángulos), con la pestaña de la otra cara.

## Comprobar un pack

`cargo test -p proxy-core --test dorados --test layouts` pinta las cartas de
prueba con el pack de la casa. Para probar uno propio, impórtalo en la app y
mira la vista previa de cada estilo: los avisos de abajo dicen si falta algo.
