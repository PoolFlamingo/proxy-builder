# Instrucciones del proyecto Proxy Builder

App de escritorio (Tauri 2 + Rust + React/TypeScript) para montar decks proxy de
Magic: The Gathering y exportarlos a un PDF listo para imprimir. Es un clon
funcional de fabricard.net (MTG Proxy Tool) como aplicación.

**El plan que manda** está en `plans/20261012 - mtg-proxy-builder/` (fuera de
git, solo en el disco local): el maestro y el sub-plan de la Fase 4 (motor de
imagen y PDF). Antes de tocar algo grande, leer el paso del plan que le toca.

## Reglas de arquitectura

- 🛑 **Todo lo pesado vive en Rust** (Pablo, 12/10/2026): parseo de la lista,
  Scryfall, descargas, caché, geometría de página, procesado de imagen, PDF y
  estadísticas. React solo pinta y guarda el estado de pantalla.
- 🛑 **Una sola geometría para pantalla y papel.** `PagePlan` (en milímetros)
  lo calcula `proxy-core::layout` y lo consumen la vista previa y el PDF. La
  pantalla nunca recalcula posiciones por su cuenta.
- 🛑 **Las imágenes de los decks viven en el almacén de blobs**
  (`app_data_dir/blobs/`, por SHA-256), no en la caché: los decks dependen de
  ellas. La caché (`app_cache_dir/renders/`) solo guarda derivados y se puede
  borrar sin perder nada.
- 🛑 **La vista previa desmonta las imágenes de las páginas que no se ven**
  (IntersectionObserver con `root` = viewport del `ScrollArea`).
- `src-tauri/crates/proxy-core` es puro: sin Tauri, sin red, sin base de datos.
  Tiene sus propios errores; la app los convierte a `AppError`.
- `src-tauri/src/lib.rs` solo arranca y registra comandos; cada área en
  `src-tauri/src/commands/<área>.rs`. Que no crezca.
- Errores: los comandos devuelven `Result<T, AppError>` (`{ code, params }`);
  la pantalla los traduce con `useLang().mensaje(e)`. Rust nunca escribe prosa
  para el usuario.
- Tipos compartidos: `#[derive(TS)] #[ts(export)]` en Rust y `pnpm bindings`
  los vuelca en `src/bindings/` (no se editan a mano).
- La pantalla habla con Rust SOLO por `src/lib/ipc.ts` (`call()`), nunca con
  `invoke` suelto.
- La vista web no sale a internet: toda red la hace Rust.

## Diseño (heredado de judlex)

Sistema de temas de judlex tal cual: `src/themes.css` (tweakcn),
`src/lib/themes.ts`, `components/theme-provider.tsx`, skins Hera/Zeus,
materiales. Radio 14 px, densidad 10 px (`p-2.5`/`gap-2.5`), texto base
`text-xs`, scroll siempre con el `ScrollArea` compartido (nunca
`overflow-auto`), las listas cuyos items llevan borde o anillo con padding
(`p-1`) para que no se corten, nada de `title=` nativo (va con `<Tooltip>`; lo vigila
`pnpm check:title`), y el scroll se prueba en WebKitGTK, no en Chrome.
El texto no se selecciona (`src/index.css`, Pablo, 20/10/2026): solo los
campos y los cuadros de copiar, que llevan `select-text`.

Textos: siempre por `t()` con la cadena fuente en español; la traducción al
inglés va en `src/i18n/en/<área>.ts`. `pnpm idiomas` falla si falta alguna.

## Herramientas

| Para qué | Comando |
|---|---|
| Arrancar | `pnpm tauri dev` |
| Todo lo barato del frontend (tsc + vitest + biome + title + idiomas + knip) | `pnpm check` |
| Rust: avisos (bloqueantes) | `pnpm check:clippy` |
| Rust: formato | `pnpm check:rust-fmt` |
| Rust: tests | `pnpm test:rust` |
| Tipos de Rust a TypeScript | `pnpm bindings` |

- **pnpm 12, no npm** (Pablo, 12/10/2026). Su configuración va en
  `pnpm-workspace.yaml`: `minimumReleaseAge: 1440` (solo versiones de hace más
  de un día, el valor de pnpm 11) y `allowBuilds` (esbuild). Regenerar el
  lockfile siempre con la versión de `packageManager` (la de Pablo): uno hecho con pnpm 10 sin esa espera trajo
  versiones recién publicadas y el `pnpm install` de Pablo lo rechazó.
- Tabulador en todo salvo Rust (cuatro espacios, `rustfmt`). En lint-staged va
  `rustfmt --edition 2021` a pelo, nunca `cargo fmt`.
- Clippy con `-D warnings`; nada de `unwrap()`/`expect()` fuera de tests
  (`#![cfg_attr(not(test), deny(...))]`).
- Usar siempre `codebase-memory-mcp` y reindexar el proyecto al cerrar cada
  fase.

## Mapa rápido

- `src/features/<área>/`: `decklist` (panel izquierdo y sus acciones),
  `preview` (hojas, casillas, menú, arrastre), `export` (panel derecho,
  alinear filas, presets), `variants` (añadir carta, variantes, rejilla de
  Scryfall), `card-editor`, `library`, `stats`, `set-browser`, `shell`,
  `disenar` (la vista «Diseñar» del editor de cartas de la 2.0).
- `src/stores/`: `deck-store` (deck, historial, autoguardado), `plan-store`
  (el `PagePlan` de Rust con 100 ms de espera), `ui-store` (zoom, selección,
  diálogos, preferencias), `trabajo-store` (el trabajo largo en marcha).
- `src/lib/trabajos.ts`: lanzar un trabajo de Rust con eventos de progreso
  (escucha antes de lanzar). `src/lib/image-url.ts`: URLs de `pbimg://`.
- Diálogos de ficheros SOLO por `src/lib/dialogos.ts` (`abrirParaImportar`,
  `guardarParaExportar`): recuerdan en Rust (`commands/carpetas.rs`,
  `app_settings`) la última carpeta de importar y la de exportar, por separado.
- Rust: `commands/{cards,decklist,export,identify,layout,library,stats}.rs`,
  `package.rs` (.pbdeck), `render.rs` + `protocol.rs` (miniaturas),
  `ocr.rs` (OCR opcional de la carta entera con `ocrs`, a la resolución
  original: título, pie —donde muchas propias llevan el nombre original— y
  resto; modelos descargados la primera vez a `app_data_dir/modelos/`),
  `scryfall/nombres.rs` (catálogo de nombres de Scryfall en `app_cache_dir`,
  renovado cada semana, para reconocer en local lo que lee el OCR),
  `proxy-core::{layout,imaging,pdf,stats,decklist}`.
- Estadísticas: todo el cálculo en `proxy-core::stats` (`mod.rs` cuenta;
  `funciones.rs` deduce rampa/robo/eliminación… por el texto de reglas;
  `mana.rs` lee costes; `prob.rs` hipergeométrica). `commands/stats.rs` pide
  por `/cards/collection` lo que falte o tenga precios de más de 24 h, y a las
  impresiones que no están en inglés les pone el precio de la inglesa (Scryfall
  no les da). El diálogo (`features/stats/`) va en pestañas y solo pinta.
- Cartas propias vinculadas: `CardSource::Custom { scryfallId?, externa? }`
  (`externa`: `RefExterna { fuente, id }`, una carta que Scryfall no tiene;
  campo opcional y no variante nueva, para que la 2.2.0 siga abriendo los
  decks). La imagen sigue siendo la propia; el vínculo da la info de carta y
  las estadísticas. `CardMeta.imageLang` es el idioma de la imagen propia.
  Una a una (`vincular-dialog`) o en bloque (`vincular-lote`: `identify_batch`
  en Rust con progreso, puntúa cada propuesta exacta/probable/ninguna y la
  pantalla la deja revisar antes de vincular; con selección, solo esas).
- **Fuentes aparte de Scryfall** (Pablo, 19/10/2026; plan
  `plans/20261019 - fuentes-idioma-release-balatro/`). Toda descarga de fuera
  va por `fuentes::Externo`: lista cerrada de sitio + ruta (`PERMITIDOS`),
  cada redirección se vuelve a comprobar, tope de tamaño leyendo por partes, y
  las imágenes siempre se decodifican y se vuelven a codificar
  (`limpiar_imagen`: JPEG 95 si eran JPEG o PNG grandes). `fuentes/cockatrice.rs`
  (fichas que Scryfall no tiene; salen en Vincular, Añadir carta y al resolver
  la lista), `fuentes/mtgio.rs` y `fuentes/mtgjson.rs` (nombres en otros
  idiomas; el índice de MTGJSON se baja en Ajustes a SQLite).
  `scryfall/fichas_auto.rs` saca las fichas de las impresiones inglesas
  (`all_parts` solo está completo ahí); «Detectar fichas» en el panel de la
  lista (`fichas-dialog`).
- OCR: `originales.rs` busca el nombre original en cualquier línea leída
  (n-gramas contra el índice local de nombres, puntuados por zona) y
  `pie_leido` saca set, número e idioma del pie.
- Exportar: el botón «Exportar PDF» va partido; los tres puntos
  (`features/export/menu-exportar.tsx`) copian la lista para Moxfield, Arena,
  MTGO o en texto plano, la guardan para MTGO (`.txt`) y guardan el
  `.pbdeck`. La lista la escribe `proxy_core::decklist::exportar` (de las
  cartas del deck; las secciones, por el nombre en la lista escrita; sin
  fichas ni huecos).
- **Marcas de carta** (Pablo, 19/10/2026): `DeckCard.marca` = variante de
  diseño, opcional o banquillo (submenú «Marca» de la casilla, insignia
  violeta). Se imprimen, pero `proxy_core::model::cuentan` las deja fuera del
  tamaño, las copias y las estadísticas (de las variantes de una carta cuenta
  una si no está sin marcar; `DeckStats.setAside` dice cuántas) y de la lista
  exportada (el banquillo va al sideboard). Al cargar la lista, la sección
  Sideboard llega como banquillo y la Maybeboard (si se incluye) como opcional.
- Idioma del deck a mano: `decks.lang_override` (migración 0004; no toca
  `updated_at`), `deck_lang_set`, submenú «Idioma» de la biblioteca.
- **Temas de tweakcn** (Pablo, 19/10/2026): `temas.rs` baja un tema por id o
  enlace (`tweakcn.com/r/themes/<id>`; la comunidad no tiene API, se abre en
  el navegador y se pega el enlace), lo pasa a CSS con el formato de
  `themes.css` (`[data-theme="tw-<id>"]`, solo variables con nombre y valor
  comprobados) y baja sus letras de Google Fonts a `app_data_dir/temas/`,
  servidas por `pbimg://…/f/<huella>.woff2`. En pantalla,
  `lib/temas-instalados.ts` los mete al final de `THEMES` y en un `<style>`
  antes de montar (`main.tsx`): `theme-provider.tsx` es de solo lectura
  (heredado de judlex) y solo acepta temas que estén en `THEMES`. El modal es
  `features/settings/temas-tweakcn.tsx` («Más temas…» en Apariencia).
- **Actualizaciones dentro de la app** (Pablo, 19/10/2026): `actualizar.rs` +
  `commands/actualizar.rs` (`tauri-plugin-updater`, solo desde Rust). Lee
  `releases/latest/download/latest.json` (solo cuentan los releases
  publicados), baja el paquete del instalador con el que se instaló
  (`.deb`/`.rpm` por `pkexec`, AppImage, NSIS, `.app`), comprueba la firma
  (`requireSignedVersion`) e instala con `request_restart`. Nunca con un
  trabajo largo en marcha (`Busy`) y guarda el deck antes. En pantalla:
  `stores/actualizacion-store.ts`, `features/shell/actualizacion-automatica.tsx`
  (al arrancar y cada 6 h si `ui-store.actualizarSola`, de fábrica encendido;
  botón en la barra; los avisos —descargando con el %, lista para reiniciar,
  fallo— los pone `useAvisosActualizacion`, se baje sola o desde Ajustes),
  el puntito del botón de Ajustes (ámbar mientras baja, del color de la app
  cuando está lista) y `features/settings/actualizaciones.tsx` (con
  «Reiniciar y actualizar» también en la cabecera de Ajustes).
  🛑 La clave privada está en `~/.tauri/proxy-builder.key` de Pablo (sin
  contraseña) y en el secreto `TAURI_SIGNING_PRIVATE_KEY`; la pública, en
  `tauri.conf.json`. Si se pierde, nadie recibe más actualizaciones.
  `pnpm release` pone las notas en el `latest.json` del borrador.
- La ventana arranca oculta y con fondo oscuro (`tauri.conf.json`) y
  `main.tsx` la enseña tras el primer fotograma (sin destello blanco); el
  window-state no guarda la visibilidad.
- **Editor de cartas (2.0).** El plan es `plans/20261014 - editor-de-cartas/`.
  - Fuera de beta desde el 19/10/2026 (Pablo): siempre montado, sin
    interruptor en Ajustes ni etiqueta «Beta».
  - Motor en `proxy-core::diseno`: `modelo` (CardDesign, CardData, Collection),
    `efectivo` (carta > colección > fábrica y la huella), `pack` (formato de
    marcos, ver `docs/packs-de-marcos.md`), `texto` y `pintar` (parley +
    tiny-skia), `render` (una cara sobre un lienzo de cualquier medida),
    `componer` (partidas, secuela, flip, batalla, meld, grandes),
    `maquetacion` (layout de la carta y cuerpo de cada cara), `estructura`
    (habilidades, capítulos, niveles… del texto) y `cuerpos` (secciones con
    insignias). Pack de la casa en `assets/marcos/builtin/`; letras en
    `assets/fuentes/`. Dorados en `tests/dorados/` y `tests/dorados/layouts/`
    (con datos reales en `layouts/datos/`, bajados por
    `borrador::en_vivo::datos_de_layouts`).
  - App: `diseno/{borrador,recursos,pintar,vista,arte,lote,png,traduccion}.rs`
    y `commands/diseno.rs`. Las imágenes de los diseños se protegen del gc
    con `blob_refs`; la vista previa va por `pbimg://…/d/<huella>/<ancho>`.
    El PDF pinta cada carta diseñada desde el diseño a su medida y PPP.
  - Cartas grandes (planos, planes, Vanguard): hojas propias al final del
    `PagePlan` (`big_grid`, `big_cells`, `Page.big`).
  - 🛑 **Marcos «Original»** (Pablo, 15/10/2026): marcos de imagen M15 de
    Card Conjurer (capas PNG del pack). No van en el repo ni en los
    instaladores: se descargan al elegirlos (`diseno/original.rs`, commit
    fijo y SHA-256 de cada PNG en `assets/marcos/original/imagenes.json`).
  - **Letras libres para descargar** (Pablo, 15/10/2026): catálogo OFL de
    Google Fonts en `src-tauri/assets/letras-libres.json` (commit fijo y
    SHA-256 de cada fichero; de título, de aire clásico/medieval/fantasía, y
    de reglas, todas legibles y sin trazos finos). `diseno/letras_libres.rs`
    las baja y las importa como letras propias; las muestras de la pantalla
    son PNG en `src/assets/letras/`. Pixelify Sans va además como alternativa
    libre a la letra del pack Balatro.
  - **Packs descargables** (Pablo, 19/10/2026): `diseno/descargables.rs` con
    el manifiesto `assets/packs-descargables.json` (commit fijo y SHA-256 por
    fichero) del repo `PoolFlamingo/proxy-builder-packs` (lo sube Pablo; al
    cambiar un pack, `python -I source/manifest.py .. <commit>`, desde la carpeta del pack, da su
    entrada nueva). El primero es **Balatro** (pixel art, 7 plantillas como estilos
    propios del pack, con sus variantes). El formato de pack versión 2
    (estilos propios, condiciones por tipo/colores, escalado pixel,
    símbolos y etiquetas propios…) está en `docs/packs-de-marcos.md`. El
    trabajo del pack (PSD → capas, `source/convert.py` y `source/build.py`) es
    ese repo, clonado al lado de este (`../proxy-builder-packs/`, Pablo,
    19/10/2026; tiene sus propias sesiones), con carpetas y ficheros en
    inglés (`layers/`, `fonts/`, `symbols/`, `labels/`, `extras/{cards,tokens,backs,lands}`);
    los ids de estilo (`tierra`, `planeta`, `espectral`, `vale`) y las claves de
    los JSON siguen en español porque los leen la app y los decks.
  - Estrechado del título y el tipo: límite (`title_squeeze_min`, 55–75 %) y
    mínimo que se aplica siempre (`title_squeeze_base`, 100 % = nada), en la
    colección o en la carta (`texto::linea_ajustada_con_base`).
  - Ajustes finos (`FineTuning`, en la colección o propios de la carta):
    altura de cada texto, tamaño de cada uno (`titleSize`… en proporción; en
    las reglas, sin valor es «automático») e interlineado de reglas y pie,
    caja de reglas más opaca, radio de la ventana de la
    ilustración (`ventanaArte` del pack) y de la carta (sin él, redondeada en
    pantalla y recta al imprimir).
  - Color del marco y caja de reglas en los marcos de imagen (Original,
    Balatro): degradado a mano = dos capas fundidas, color propio = la capa
    neutra teñida (`render::ExtraMarco`), y la caja pedida se pinta en las
    capas `cajaReglas` del pack. La definición de los marcos Original en
    disco se pone al día al cargar (`original::definicion_al_dia`).
  - Vista previa en tiempo real: `lib/imagen-viva.ts` (se pide al ritmo del
    cambio, la última gana, más pequeña mientras se arrastra).
  - Mover la ilustración (arrastre, rueda, deslizadores del encuadre) no
    espera a Rust: `design_overlay` pinta una vez la carta sin ilustración
    (`render_capa`, solo caras normales) y dice dónde va el hueco;
    `disenar/arte-en-vivo.tsx` pone debajo la ilustración (`pbimg://…/b/`)
    con la misma cuenta que `pintar_arte` (`disenar/encuadre.ts`). Al parar
    entra la carta exacta. Las compuestas y grandes siguen por el camino normal.
- **Biblioteca** (`features/library/`): etiquetas del usuario (`store/etiquetas.rs`,
  tablas `tags` y `deck_tags`), favoritos, notas y portada elegida
  (`decks.pinned/notes/cover_card_id`, migración 0003; no tocan `updated_at`).
  Los detalles de cada deck (colores, clase, precio, idiomas, formatos…) los
  calcula `biblioteca.rs` sin red con la caché de Scryfall y los guarda en
  memoria por `updated_at`. En pantalla: `organizar.ts` (filtrar, ordenar,
  agrupar —por etiqueta un deck sale en cada una— y selección, con tests),
  `store.ts`, `preferencias.ts` (vista, orden y grupos se recuerdan; los
  filtros no), cuadrícula y lista (columnas según el ancho), barra lateral de
  etiquetas, menú del deck compartido (desplegable y clic derecho; con varios
  elegidos actúa sobre todos) y el diálogo de propiedades. El `.pbdeck` lleva
  etiquetas y notas.
- «Editar carta» aplica a la carta, a sus copias (misma imagen: todo) o a
  todas (solo el color): `features/card-editor/propagar.ts`.
- El `README.md` (en inglés) es la presentación de la app (Pablo, 14/10/2026):
  qué hace, con capturas en `docs/presentacion/` sacadas de la app en Xvfb con
  un deck de ejemplo; lo de desarrollar va al final. `README.es.md` es su copia
  en español (Pablo, 15/10/2026): cualquier cambio va a los dos, con las mismas
  secciones y capturas. Las del inglés salen de la app en inglés y van en
  `docs/presentacion/en/` con los mismos nombres; las del español, en
  `docs/presentacion/`.
- Idioma de fábrica de la app: inglés (Pablo, 15/10/2026); el español se elige
  en Ajustes y se recuerda (`language-provider`, `proxybuilder.lang`).
- Releases: los hace Pablo (o Claude Code en su VS Code), no esta sesión.
  El texto de la release: `--notas "…"`, `--notas-fichero <ruta>` o
  `--sin-notas`; sin ninguno, abre `$EDITOR` (`scripts/release-notas.mjs`).
- Valores de fábrica (Pablo, 13/10/2026): 2 mm de separación en blanco,
  sangrado 0, guías de corte apagadas y «Rellenar las esquinas» apagado (lo
  transparente sale del color del papel; `corners::fill` solo si se activa).
- 🛑 Encuadre (Pablo, 13/10/2026): la imagen se **estira** al hueco exacto
  (`imaging::frame`), aunque se deforme un poco; solo se recorta si la
  proporción se aleja más de un 15 % (`MAX_ESTIRADO`).
- 🛑 Los modales son 100 % opacos con cualquier material o skin (regla al
  final de `src/index.css`).

## Forma de trabajo

Se programa y se pasan los tests en la máquina de la nube (con WebKitGTK
instalado para compilar el crate de la app), se sincroniza a
`/home/estigio/Projects/Tauri/proxy-builder` y Pablo arranca la app en su
equipo. Los planes (`plans/`) no se versionan.

- 🛑 **Los cambios llegan como ediciones sin commit** (Pablo, 14/10/2026): él
  quiere verlos en su árbol de trabajo y hacer los commits él. En la nube no
  se hace commit: se saca un parche (`git diff --binary` desde lo último que
  se le mandó), se copia a su `tmp/` y se aplica allí con `git apply` (sin
  commit). Su equipo no tiene cargo ni pnpm: todo se compila y se prueba
  aquí antes de mandarlo.
- 🛑 **No subir a GitHub desde la nube** (Pablo, 14/10/2026): cada push lanza
  el CI y le llegan correos si falla. Él decide cuándo hacer commit, subir y
  sacar versión. Antes de mandar un parche, lo mismo que el CI
  (`pnpm bindings`, `pnpm check`, `pnpm check:rust-fmt`, `pnpm check:clippy`
  con el clippy de la última estable y `pnpm test:rust`), con
  `biome check --write` y `cargo fmt` pasados antes (lint-staged lo haría al
  hacer commit).
- Probar la ventana en la nube: `Xvfb :99` y la app con
  `WEBKIT_DISABLE_COMPOSITING_MODE=1 WEBKIT_DISABLE_DMABUF_RENDERER=1`. Sin
  eso, en una pantalla sin GPU las animaciones de entrada de los diálogos no
  avanzan y quedan invisibles (pero abiertos y tapando los clics). En el
  equipo de Pablo no hace falta.
