# Proxy Builder

Aplicación de escritorio para montar decks proxy de Magic: The Gathering a partir
de una lista de cartas, revisarlos página a página tal como saldrán impresos y
exportarlos a un PDF listo para imprimir (también a doble cara).

Tauri 2 + Rust + React/TypeScript. Linux, Windows y macOS.

## Requisitos

- Node 22 y pnpm (`corepack enable` lo activa).
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

## Créditos

Los datos y las imágenes de las cartas vienen de [Scryfall](https://scryfall.com).
Magic: The Gathering es una marca de Wizards of the Coast. Esta aplicación no está
afiliada ni respaldada por ninguna de las dos.
