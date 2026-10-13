# Proxy Builder

Aplicación de escritorio para montar decks proxy de Magic: The Gathering a partir
de una lista de cartas, revisarlos página a página tal como saldrán impresos y
exportarlos a un PDF listo para imprimir (también a doble cara).

Tauri 2 + Rust + React/TypeScript. Linux, Windows y macOS. Interfaz en
español e inglés.

## Qué hace

- **Lista de cartas**: pegar una lista (Arena, MTGO, Moxfield, Archidekt…),
  importar un deck público de Archidekt por URL o explorar un set entero. Las
  líneas que no se entienden salen marcadas.
- **Scryfall**: idioma de las cartas con vuelta al inglés si no existe,
  política de imagen (con imagen, la mejor, solo alta resolución), tokens
  automáticos y cambio de variante por carta o por nombre.
- **Vista previa exacta**: las hojas se pintan con la misma geometría que el
  PDF. Solo se cargan las imágenes de las páginas cercanas a la vista.
- **Cartas propias y reversos**: subir imágenes o arrastrarlas desde el gestor
  de ficheros, reverso por defecto propio o el genérico de la app.
- **Editor de carta**: giro, encuadre y once retoques (brillo, contraste,
  nitidez…), calculados en Rust igual que en el PDF.
- **PDF**: 300/600/900 PPP, JPEG o PNG, separación, sangrado, guías de corte,
  oscurecido de bordes, doble cara (borde largo o corto) con hoja de
  calibración y alineado por filas, partir cada N hojas.
- **Biblioteca**: los decks se guardan solos con sus imágenes; se pueden
  duplicar, renombrar y llevar a otro equipo como `.pbdeck`.
- **Estadísticas**: curva de maná, colores, tipos, rareza, precio estimado y
  legalidad.

## Requisitos

- Node 22 y pnpm 12 o superior.
- Rust estable (1.92 o superior).
- En Linux, las librerías de Tauri:
  `sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`

## Arrancar

```bash
pnpm install
pnpm tauri dev
```

La primera compilación de Rust tarda varios minutos.

## Comprobar

```bash
pnpm check            # TypeScript, tests, Biome, textos y Knip
pnpm check:clippy     # avisos de Rust (bloqueantes)
pnpm test:rust        # tests de Rust
```

## Instaladores

`pnpm tauri build` genera los de la plataforma en la que se ejecuta. El
workflow `.github/workflows/release.yml` genera los de Linux, Windows y macOS
(Intel y Apple Silicon) en un borrador de release. Para sacar una versión:
`pnpm release patch` (o `minor`, `major`, `X.Y.Z`; con `--dry` solo comprueba).
Sube la versión en los cuatro sitios, la commitea y la sube, espera a la CI,
lanza el workflow a mano (pestaña Actions → release) y, al terminar, pregunta si
publicar el borrador. Al publicarlo se crea la etiqueta. Necesita `gh` con la
sesión iniciada.

Los instaladores no van firmados: Windows (SmartScreen) avisará de «editor
desconocido» (Más información → Ejecutar de todas formas) y macOS (Gatekeeper)
pedirá abrirla con clic derecho → Abrir la primera vez.

Los datos viven en la carpeta de datos de la app (`com.estigio.proxybuilder`):
la biblioteca (`library.db`), las imágenes de los decks (`blobs/`) y el
registro (`logs/proxy-builder.log`). La caché de miniaturas se puede borrar
desde la app sin perder nada.

## Créditos

Los datos y las imágenes de las cartas vienen de [Scryfall](https://scryfall.com).
Magic: The Gathering es una marca de Wizards of the Coast. Esta aplicación no está
afiliada ni respaldada por ninguna de las dos.
