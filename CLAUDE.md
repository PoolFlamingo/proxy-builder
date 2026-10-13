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
`overflow-auto`), nada de `title=` nativo (va con `<Tooltip>`; lo vigila
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
- 🟡 Temporal: `knip.json` ignora `src/components/ui/**`, `src/lib/ipc.ts` y
  varias dependencias que aún no usa nadie. Se quita al cerrar la Fase 6.
- Usar siempre `codebase-memory-mcp` y reindexar el proyecto al cerrar cada
  fase.

## Forma de trabajo

Se programa y se pasan los tests en la máquina de la nube (con WebKitGTK
instalado para compilar el crate de la app), se sincroniza a
`/home/estigio/Projects/Tauri/proxy-builder` y Pablo arranca la app en su
equipo. Los planes (`plans/`) no se versionan.
