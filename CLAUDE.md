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
- Cartas propias vinculadas: `CardSource::Custom { scryfallId? }`. La imagen
  sigue siendo la propia; el vínculo da la info de carta y las estadísticas.
  Una a una (`vincular-dialog`) o en bloque (`vincular-lote`: `identify_batch`
  en Rust con progreso, puntúa cada propuesta exacta/probable/ninguna y la
  pantalla la deja revisar antes de vincular; con selección, solo esas).
- **Editor de cartas (2.0).** El plan es `plans/20261014 - editor-de-cartas/`.
  - 🛑 Experimental y **apagado de fábrica** (Pablo, 14/10/2026): se enciende en
    Ajustes (`ui-store.editorCartas`, `setEditorCartas`). Apagado no se monta
    nada suyo: ni la pestaña «Diseñar», ni sus diálogos (`App.tsx`), ni sus
    entradas del menú de la casilla. Las cartas ya diseñadas se siguen
    imprimiendo desde su diseño.
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
    son PNG en `src/assets/letras/`.
  - Estrechado del título y el tipo: límite (`title_squeeze_min`, 55–75 %) y
    mínimo que se aplica siempre (`title_squeeze_base`, 100 % = nada), en la
    colección o en la carta (`texto::linea_ajustada_con_base`).
  - Vista previa en tiempo real: `lib/imagen-viva.ts` (se pide al ritmo del
    cambio, la última gana, más pequeña mientras se arrastra).
  - Mover la ilustración (arrastre, rueda, deslizadores del encuadre) no
    espera a Rust: `design_overlay` pinta una vez la carta sin ilustración
    (`render_capa`, solo caras normales) y dice dónde va el hueco;
    `disenar/arte-en-vivo.tsx` pone debajo la ilustración (`pbimg://…/b/`)
    con la misma cuenta que `pintar_arte` (`disenar/encuadre.ts`). Al parar
    entra la carta exacta. Las compuestas y grandes siguen por el camino normal.
- «Editar carta» aplica a la carta, a sus copias (misma imagen: todo) o a
  todas (solo el color): `features/card-editor/propagar.ts`.
- El `README.md` es la presentación de la app (Pablo, 14/10/2026): qué hace,
  con capturas en `docs/presentacion/` sacadas de la app en Xvfb con un deck
  de ejemplo; lo de desarrollar va al final.
- Releases: los hace Pablo (o Claude Code en su VS Code), no esta sesión.
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
