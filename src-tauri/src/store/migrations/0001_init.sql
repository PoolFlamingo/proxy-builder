-- Esquema inicial de la biblioteca local de Proxy Builder.
-- 🛑 Las migraciones solo AÑADEN: nunca se edita una ya publicada.

CREATE TABLE decks (
    id                TEXT PRIMARY KEY,
    name              TEXT NOT NULL,
    decklist_text     TEXT NOT NULL DEFAULT '',
    settings_json     TEXT NOT NULL,
    default_back_json TEXT,
    default_back_blob TEXT,
    schema_version    INTEGER NOT NULL,
    created_at        INTEGER NOT NULL,
    updated_at        INTEGER NOT NULL
);

-- Una fila por copia física. El resto de la carta va en `card_json`; las
-- columnas de blobs existen para saber qué imágenes usa cada deck sin leer
-- el JSON (limpieza de huérfanas).
CREATE TABLE deck_cards (
    id         TEXT NOT NULL,
    deck_id    TEXT NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
    position   INTEGER NOT NULL,
    front_blob TEXT,
    back_blob  TEXT,
    card_json  TEXT NOT NULL,
    PRIMARY KEY (deck_id, id)
);
CREATE INDEX deck_cards_orden ON deck_cards(deck_id, position);
CREATE INDEX deck_cards_front ON deck_cards(front_blob);
CREATE INDEX deck_cards_back ON deck_cards(back_blob);

CREATE TABLE blobs (
    hash       TEXT PRIMARY KEY,
    ext        TEXT NOT NULL,
    mime       TEXT NOT NULL,
    width      INTEGER NOT NULL,
    height     INTEGER NOT NULL,
    bytes      INTEGER NOT NULL,
    origin     TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE presets (
    id         TEXT PRIMARY KEY,
    kind       TEXT NOT NULL,
    name       TEXT NOT NULL,
    json       TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX presets_kind ON presets(kind, name);

CREATE TABLE app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Caché de metadatos de Scryfall (Fase 3).
CREATE TABLE scryfall_cards (
    id               TEXT PRIMARY KEY,
    oracle_id        TEXT,
    name             TEXT NOT NULL,
    lang             TEXT NOT NULL,
    set_code         TEXT NOT NULL,
    collector_number TEXT NOT NULL,
    json             TEXT NOT NULL,
    fetched_at       INTEGER NOT NULL
);
CREATE INDEX scryfall_nombre ON scryfall_cards(name, lang);
CREATE INDEX scryfall_impresion ON scryfall_cards(set_code, collector_number, lang);
