-- La biblioteca organizada: etiquetas del usuario, favoritos, notas, portada
-- elegida y cuándo se abrió cada deck por última vez.
ALTER TABLE decks ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
ALTER TABLE decks ADD COLUMN notes TEXT NOT NULL DEFAULT '';
ALTER TABLE decks ADD COLUMN opened_at INTEGER;
-- La carta de la portada (id de una copia del deck); sin ella, la primera.
ALTER TABLE decks ADD COLUMN cover_card_id TEXT;

CREATE TABLE tags (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    color      TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE UNIQUE INDEX tags_name ON tags(name COLLATE NOCASE);

CREATE TABLE deck_tags (
    deck_id TEXT NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
    tag_id  TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (deck_id, tag_id)
);
CREATE INDEX deck_tags_tag ON deck_tags(tag_id);
