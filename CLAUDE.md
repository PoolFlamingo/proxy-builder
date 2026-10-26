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

Textos: siempre por `t()` con la cadena fuente en español; la traducción va
en **los 15 diccionarios** `src/i18n/<código>/<área>.json` (`es`, la fuente, que repite cada cadena; `en`, `fr`, `de`,
`it`, `pt`, `ro`, `pl`, `nl`, `ru`, `uk`, `ja`, `ko`, `zhs`, `zht`; Pablo,
21/10/2026), con los términos de `src/i18n/GLOSARIO.md`. Una cadena con
`{n}` que cuenta va como plural (`{ one, few, many, other }`, las categorías
de `Intl.PluralRules` de cada idioma) y `n` se pasa como número, sin
formatear. `pnpm idiomas` falla si falta alguna en cualquier idioma, si un
`{x}` se pierde o si a un plural le falta una forma. La tabla de idiomas
(código, nombre propio, bandera, locale, si hay cartas impresas) es
`src/lib/idiomas.ts`; fechas y números, con `localeDe(lang)`. Al idioma que
le falta una cadena le sale la inglesa.

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
  fase. 🛑 Para explorar código (buscar símbolos, llamadas, texto) van
  `search_graph`, `search_code`, `trace_path` y `get_code_snippet`, **no**
  `grep`/`rg`/`find` por Bash (Pablo, 20/10/2026); `grep` solo para lo que no
  es código (configs, JSON, markdown) o si el grafo no devuelve nada.

## Mapa rápido

- `src/features/<área>/`: `decklist` (panel izquierdo y sus acciones),
  `preview` (hojas, casillas, menú, arrastre), `export` (panel derecho,
  alinear filas, presets), `variants` (añadir carta, variantes, rejilla de
  Scryfall), `card-editor`, `library`, `stats`, `set-browser`, `shell`,
  `disenar` (la vista «Diseñar» del editor de cartas de la 2.0), `tutorial`
  (el recorrido guiado de cada pantalla: pasos con su ancla `data-tutorial`;
  sale solo la primera vez y con el botón de ayuda de la barra), `mesa`
  (la pestaña «Jugar»: entrada, tablero, menús, diálogos, atajos).
- `src/stores/`: `deck-store` (deck, historial, autoguardado), `plan-store`
  (el `PagePlan` de Rust con 100 ms de espera), `ui-store` (zoom, selección,
  diálogos, preferencias, `mesa` = Ajustes > Mesa), `trabajo-store` (el
  trabajo largo en marcha), `mesa-store` (la vista del motor y sus parches;
  selectores con referencias estables: `NINGUNA` o `useShallow`).
- `src/lib/trabajos.ts`: lanzar un trabajo de Rust con eventos de progreso
  (escucha antes de lanzar). `src/lib/image-url.ts`: URLs de `pbimg://`.
- Diálogos de ficheros SOLO por `src/lib/dialogos.ts` (`abrirParaImportar`,
  `guardarParaExportar`): recuerdan en Rust (`commands/carpetas.rs`,
  `app_settings`) la última carpeta de importar y la de exportar, por separado.
- **Mesa de juego** (Pablo, 24/10/2026; plan `plans/20261024 - mesa-de-juego/`).
  Motor puro en `src-tauri/crates/mesa` (sin Tauri, red, base de datos, reloj
  ni azar de fuera; nunca `HashMap` en el modelo): `Intencion` → `resolver`
  (permisos y azar: semilla por evento) → `Evento` → `aplicar` (total: sobre
  una copia, `Err` no cambia nada) → `vista`/`parche` por mirador (de una
  zona oculta solo el número). `registro.rs` deshace con `Cambio::Restaurar`
  (la partida entera), rehace re-resolviendo intenciones y repite con fotos
  cada 50. 🛑 `Accion`/`Cambio` nunca se cambian ni se quitan: variante nueva
  = subir `VERSION_CATALOGO` y escribir `tests/catalogo/v<N>.json`.
  La app: `src-tauri/src/mesa/` (`preparar.rs` del deck a la mesa, con las
  imágenes a 745 px en `app_cache_dir/mesa/` servidas por `pbimg://…/m/`;
  `sesion.rs` la partida abierta, que manda vista y parches por un
  `tauri::ipc::Channel`; `guardada.rs` en `app_data_dir/partidas/actual.json.gz`
  cada 50 eventos, a los 20 s con algo sin guardar y al salir; `tapete.rs`
  con `blob_refs` de tipo `mesa`) y `commands/mesa.rs`. La pantalla nunca
  decide reglas: manda `Accion` por `mesa_accion` y pinta lo que vuelve; el
  registro lo escribe `features/mesa/textos-eventos.ts` con `t()`. Un
  rechazo llega como `Rejected` con `motivo` (`lib/errores.ts`). Con la mesa
  abierta no se instala una actualización (`Busy`, `motivo: partida`).
  **Listas para jugar** (Pablo, 26/10/2026): un deck con más cartas de las
  que se juegan (variantes de diseño, alternativas) elige una lista: el
  comandante (una copia concreta) y 99 más. `mesa::lista` (dato y
  comprobación: tamaño, copias salvo básicas, identidad de color; solo
  avisa de la identidad), tabla `deck_listas` (migración 0005, aparte del
  deck como las etiquetas: el autoguardado no la pisa; viaja en el
  `.pbdeck`), `store/listas.rs`, comandos `mesa_lista*` y
  `features/mesa/listas/`. Commander con más de 100 que cuentan no empieza
  sin lista (`motivo: falta_lista`).
  **Sparring** (Fase D): el rival fantasma del solitario, como el de
  Playgroup (tablas, no IA). `mesa::sparring::reaccionar(antes, después,
  evento, azar)` (contrarresta al lanzar, elimina/devuelve/sacrificio al
  llegar al campo, barre al pasar con criaturas; de las tierras solo quita las muy fuertes —legendarias o `TIERRAS_FUERTES`—, y las rocas y elfos de maná pesan menos; Pablo, 26/10/2026); la sesión mete la reacción
  como `Cambio::Sparring` de la mesa y el jugador decide con
  `DecidirSparring`. Bases, ritmo e intensidades arriba de
  `sparring/reacciones.rs`, calibradas con `tests/sparring.rs` (500
  partidas por intensidad, reparto de Playgroup ±5 puntos); las 128 cartas
  del fantasma en `mesa/assets/sparring/cartas.json`, por nombre
  (`mesa_sparring_carta` las busca en Scryfall). Pantalla en
  `features/mesa/sparring/`.
  **Dados y monedas** (`features/mesa/dados/`): el resultado lo decide Rust
  y la animación solo lo enseña. Tres niveles (Ajustes > Mesa): 3D con
  three.js y física de Rapier determinista (`fisica.ts` simula entera la
  tirada, mira qué cara quedó arriba y `conResultado` le pone el número del
  motor), CSS 3D low poly (`css3d.tsx`, la pose final ya calculada) y 2D
  (`plano.tsx`). three y Rapier van con `import()` en trozos aparte; Rapier
  necesita `'wasm-unsafe-eval'` en la CSP. El aspecto de cada jugador
  (color, números, acabado, imagen propia, metal de la moneda; Pablo,
  26/10/2026) lo guarda `mesa/dados.rs` (`app_settings` `mesa.dados`, la
  imagen en blobs con `blob_refs` `mesa`/`dados`, a la pantalla como `data:`
  URL para que WebGL pueda usarla); `texturas.ts` pinta el fondo de las
  caras para los tres niveles. Sonido sintetizado (`sonidos.ts`, Web Audio,
  sin ficheros) con los golpes que graba la física; los demás sonidos, en la
  Fase H. 🛑 Nunca coger `state.db.conn()` y llamar con ella cogida a algo
  que la vuelva a pedir (`settings::set`…): el `Mutex` se bloquea para
  siempre.
  **En red** (Fases E y F): iroh 1.4.1 (QUIC P2P con relé), en estrella
  con el host: los invitados mandan intenciones (ALPN `proxy-builder/mesa/1`,
  tramas de `red/protocolo.rs`), el host resuelve y reparte eventos con la
  huella cada 50; cada jugador guarda la partida entera, el espectador solo
  su vista (`red/mirada.rs`, `AppState.mirada`). Imágenes por `/img/1` a
  través del host (`red/imagenes.rs`, `app_cache_dir/mesa/r/`, se limpian
  siempre). La clave del nodo en `app_data_dir/identidad/clave`;
  `PB_PERFIL` (debug) separa varias instancias en la misma máquina
  (`perfiles/<x>/`). Código directo: comandos `red_*`
  (`commands/red.rs`, `features/mesa/red/`, `stores/red-store.ts`).
  Servidor de salas y playgroups: cliente en `red/servidor/` (WebSocket
  con rustls y los certificados del sistema, nada de OpenSSL; reto firmado
  con la clave del nodo; `sala.rs` hace la malla, mide RTT y cambia de host
  cuando el servidor lo elige), comandos `salas_*`, pantalla en
  `features/salas/` y `stores/salas-store.ts`. El servidor (Node ≥ 22.13,
  `node:sqlite`, `ws`, `zod`) es un repo aparte, `proxy-builder-server`,
  clonado en `server/` (ignorado por la app; su protocolo en
  `server/docs/protocolo.md` y el despliegue con nginx e `iroh-relay`
  1.4.1 en `server/docs/despliegue.md`). Mientras no haya dominio,
  `PRODUCCION` (`red/servidor/mod.rs`) y `RELE_PROPIO` (`red/nodo.rs`) son
  `None`; en debug, `PB_SERVIDOR` y `PB_RELAY` los cambian.
- Rust: `commands/{cards,decklist,export,identify,layout,library,stats}.rs`,
  `package.rs` (.pbdeck), `render.rs` + `protocol.rs` (miniaturas),
  `ocr.rs` (OCR opcional de la carta entera con `ocrs`, a la resolución
  original: título, pie —donde muchas propias llevan el nombre original— y
  resto; modelos descargados la primera vez a `app_data_dir/modelos/`),
  `scryfall/nombres.rs` (catálogo de nombres de Scryfall en `app_cache_dir`,
  renovado cada semana, para reconocer en local lo que lee el OCR),
  `proxy-core::{layout,imaging,pdf,stats,decklist}`, `mesa`.
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
- **Salir con cosas a medias pregunta.** Cerrar la ventana
  (`features/shell/confirmar-cierre.tsx`, `onCloseRequested`) guarda el deck y,
  si hay un trabajo largo, una actualización bajando, la carta que se diseña
  sin guardar o un diálogo con algo en curso o sin guardar, frena el cierre y
  pregunta (`lib/cierre.ts`; con la carta sin guardar, «Guardar y cerrar»).
  Cambiar de carta o de pantalla desde Diseñar pregunta igual y ofrece
  «Guardar y continuar» (`Confirmar` con `accionAlternativa`;
  `stores/edicion-store.ts` dice cómo guardar la carta). Un diálogo con un
  proceso o cambios propios lo declara con `useEstadoParaCerrar`
  (`lib/dialogos-abiertos.ts`): `components/ui/` es de solo lectura. 🛑 Cerrar
  lo hace `destroy()`, con el permiso `core:window:allow-destroy`
  (`capabilities/default.json`): sin él la ventana no se cierra.
- La ventana arranca oculta y con fondo oscuro (`tauri.conf.json`) y
  `main.tsx` la enseña tras el primer fotograma (sin destello blanco); el
  window-state no guarda la visibilidad.
- **Editor de cartas (2.0).** El plan es `plans/20261014 - editor-de-cartas/`.
  - Fuera de beta desde el 19/10/2026 (Pablo): siempre montado, sin
    interruptor en Ajustes ni etiqueta «Beta».
  - **Idiomas de diseño** (Pablo, 21/10/2026; plan
    `plans/20261021 - idiomas-y-emoji/`): `DesignLang` tiene los 15 de la
    interfaz. `ByLang` guarda `es` y `en` siempre y los demás solo si
    existen (`otros`), así un diseño es/en se guarda byte a byte como antes
    y su huella no cambia (test `las_huellas_en_espanol_e_ingles_son_las_de_antes`
    y `tests/compatibilidad/diseno-2.5.9.json`). En la pantalla se leen con
    `lib/por-idioma.ts` (`ts-rs` solo ve `es`/`en`). Nombres impresos y «sin
    texto» de los otros idiomas en `FaceData.printed_names` y
    `CardData.sin_texto` (por código; `printed_name_es` y `sin_texto_es`
    siguen igual para la 2.5.x). La app baja el español siempre y además el
    idioma de diseño del deck (`borrador::datos_de`, `anadir_idioma`); al
    cambiarlo, `design_complete_lang` (`diseno/idioma.rs`) completa las
    cartas que lo necesiten. `ro`, `pl`, `nl`, `uk` no tienen cartas: reglas
    en inglés y tipo de `proxy-core::diseno::tipos_idiomas` (tablas sacadas de
    cartas reales; las de esos cuatro, sin fuente oficial). Las palabras fijas
    («NIVEL», Fusionar, el código del pie) en `textos_motor.rs`. Japonés,
    coreano y chino: Noto Sans JP/KR/SC/TC bajadas aparte
    (`diseno/letras_cjk.rs`, `assets/letras-cjk.json`, a
    `app_data_dir/letras-cjk/`); sin ellas, aviso `FaltanLetras`. Lo que a
    una letra le falte de latín o cirílico sale con Spectral o Montserrat
    (`Letras::respaldos`).
  - Motor en `proxy-core::diseno`: `modelo` (CardDesign, CardData, Collection),
    `efectivo` (carta > colección > fábrica y la huella), `pack` (formato de
    marcos, ver `docs/packs-de-marcos.md`), `texto` y `pintar` (parley +
    tiny-skia), `render` (una cara sobre un lienzo de cualquier medida),
    `componer` (partidas, secuela, flip, batalla, meld, grandes),
    `maquetacion` (layout de la carta y cuerpo de cada cara), `estructura`
    (habilidades, capítulos, niveles… del texto, y el plano de las mazmorras:
    salas pegadas con sus puertas, como las impresas), `cuerpos` (secciones con
    insignias), `emoji` (emoji en color: Noto Color Emoji COLRv1 incrustada,
    familia genérica de emoji de parley, capas, degradados y modos de fusión
    pintados con `ColorPainter` de skrifa; Pablo, 20 y 21/10/2026),
    `textos_motor` (palabras fijas por idioma) y `tipos_idiomas` (líneas de
    tipo sin red). Pack de la casa en `assets/marcos/builtin/`; letras en
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
    fijo y SHA-256 de cada PNG en `proxy-core/assets/marcos/original/imagenes.json`).
    Clásico, extendido y despejado llevan el mismo mármol y pergamino (Pablo,
    21/10/2026): piezas del marco normal sobre los otros con `mascara` y
    `desde` de las capas PNG (pack versión 3). Si una versión nueva pide
    imágenes que no están, carga sin esos estilos (`original::cargar`) y las
    baja sola al arrancar (`original::completar`).
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
- Idioma de fábrica de la app: inglés (Pablo, 15/10/2026); los otros 14 se
  eligen en Ajustes (`components/selector-idioma.tsx`, con bandera) y se
  recuerdan (`language-provider`, `proxybuilder.lang`). Cada diccionario es un
  trozo aparte que se carga al elegirlo (`i18n/cargar.ts`); `main.tsx` lo
  espera antes del primer pintado.
- **Emoji de Noto** (Pablo, 21/10/2026): en la interfaz, los SVG de Noto Color
  Emoji que usa la app (las banderas de los idiomas) en `src/assets/emoji/`
  (los copia `scripts/emoji-noto.mjs`, commit fijo y SHA-256 en
  `fuentes.json`; la de España, en PNG) y se pintan con `<Emoji>` /
  `<Bandera>` (`components/emoji.tsx`). En las cartas, `Noto-COLRv1.ttf`
  incrustada (`render.rs`, `LETRA_DE_EMOJI`) y pintada con sus degradados y
  capas (`diseno/emoji.rs`); solo las cartas con emoji cambian de huella
  (`|emoji-noto`).
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

## Wiki (documentación)

La documentación completa del proyecto vive en el wiki de GitHub
(`https://github.com/PoolFlamingo/proxy-builder/wiki`), **en inglés (idioma de
referencia) y en español**. Es un
repo aparte (`PoolFlamingo/proxy-builder.wiki`) con su clon local en
`/home/estigio/Projects/Tauri/proxy-builder.wiki/`, al lado de este y del de
packs. No es lo mismo que `README.md`/`README.es.md` (la presentación) ni que
`docs/packs-de-marcos.md`: el wiki es la referencia para entender, ampliar y
rehacer la app.

- 🛑 **Todo cambio que añada, modifique o quite algo se documenta en el wiki en
  la misma sesión, antes de darlo por terminado** (Pablo, 20/10/2026): una
  función de cara al usuario, un comando de Rust, un ajuste, un formato
  (`.pbdeck`, packs, migración de la base de datos), una regla de arquitectura,
  un cambio en el CI o en la release, una carpeta de datos nueva… Se actualizan
  las páginas afectadas; si el cambio no cabe en ninguna, página nueva.
- Qué página toca qué área: el mapa está en `Documentation.md` y `File-Index.md` del
  wiki (cada área de `src/features/*`, `src-tauri/src/*` y
  `proxy-core::*` tiene su página). Los cambios de «Mapa rápido» de este fichero
  se reflejan también allí.
- Página nueva: se enlaza desde `Documentation.md`, `Documentación.md` y `_Sidebar.md`, y desde
  las páginas relacionadas (cada una acaba con `## See also`; en la española,
  `## Véase también`).
- 🛑 **Idiomas** (Pablo, 20/10/2026): GitHub no tiene selector de idioma en los
  wikis, así que cada página existe una vez por idioma: la inglesa
  (`User-Guide.md`, la de referencia) y la española con **su título traducido
  como nombre de página, sin prefijos** (`Guía-de-usuario.md`; GitHub saca el
  título de la página del nombre del fichero, por eso no se usa `ES-`). Justo
  bajo el `# Título` va la barra `**English** · [Español](Guía-de-usuario)` (en
  la española, `[English](User-Guide) · **Español**`), y `_Sidebar.md` pone en
  cada línea los dos enlaces (`- [User-Guide](User-Guide) · [Guía de
  usuario](Guía-de-usuario)`); el índice español es `Documentación.md` (la portada inglesa, `Documentation.md`). El nombre de
  la española sale del `# Título` traducido, con guiones y sin los caracteres
  `\ / : * ? " < > | # % & { } +`. **Un cambio se escribe primero en inglés y
  se traslada a todas las versiones en el mismo trabajo**; nunca se deja una
  versión desfasada ni se sube un idioma sin los demás. En las traducciones no
  se traduce código, rutas, símbolos, errores ni textos de la interfaz ya
  citados; en la española los textos de interfaz son los de la fuente en
  español; los enlaces a `Pagina#ancla` pasan a la página sin ancla (los
  títulos traducidos cambian el ancla); las capturas de la española salen de
  `docs/presentacion/` y las de la inglesa de `docs/presentacion/en/`;
  `Inferred:` pasa a `Deducido:`; el texto de cada enlace es el título de la
  página destino en ese idioma. Idioma nuevo = una versión de cada página con su
  título traducido, su portada (`Documentación` en español), la barra de idioma en todas
  y su enlace en cada línea de `_Sidebar.md`. Si se renombra una página, se
  corrigen los enlaces de todas y la barra lateral.
- Formato: ficheros planos `Nombre-Con-Guiones.md`, enlaces `[Texto](Nombre-Pagina)`
  (sin `.md`), diagramas en Mermaid, rutas y símbolos reales entre comillas
  invertidas. Describe el código **tal como está ahora** y, sobre todo, **el
  porqué** (decisiones, invariantes, casos límite), sin fechas ni nombres de
  persona. Lo que no se sabe con certeza va marcado como `Inferred:`.
- Nunca en el wiki: claves, rutas de claves, rutas absolutas del equipo, correos
  ni nada de `plans/` (no se versiona).
- Antes de subir: `git pull` en el clon del wiki, comprobar que no hay enlaces
  rotos entre páginas ni rutas del repo que ya no existen, `git commit` y
  `git push`. **El wiki sí se sube** (Pablo, 20/10/2026): no lanza el CI ni manda
  correos, a diferencia de este repo, donde sigue valiendo la regla de no subir.
  Lo que sí se mantiene: el código llega a Pablo como edición sin commit y el
  wiki se sube al terminar la tarea; en el mensaje final se dice qué páginas se
  tocaron.
- Si el wiki aún no existe en GitHub (no se puede clonar), se escribe igualmente
  en el clon local y se avisa a Pablo para que cree la primera página desde la
  web; después se vuelca todo.
- Al cerrar cada fase (junto a reindexar `codebase-memory-mcp`), repasar el wiki
  entero contra el código por si algo quedó desfasado.
