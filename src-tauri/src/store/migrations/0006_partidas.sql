-- El historial de la mesa de juego (Fase H): cada partida terminada, con su
-- resultado y, si quien la guarda tuvo la partida entera, su registro
-- (comprimido) para poder repetirla. No cuelga de los decks: si se borra el
-- deck, sus partidas se quedan con el nombre que tenía. Aparte del deck, como
-- las listas: jugar no cambia `updated_at`.
CREATE TABLE partidas (
    id               TEXT PRIMARY KEY,
    empezada         INTEGER NOT NULL,
    terminada        INTEGER NOT NULL,
    -- JSON de cada valor (`"commander"`, `"combate"`…).
    formato          TEXT NOT NULL,
    modo             TEXT NOT NULL,
    -- JSON: [{jugador, nombre, color, deckId, deck, puesto}]
    jugadores        TEXT NOT NULL,
    -- JSON: asientos ganadores.
    ganadores        TEXT NOT NULL,
    como             TEXT NOT NULL,
    nota             TEXT NOT NULL DEFAULT '',
    rondas           INTEGER NOT NULL,
    duracion_ms      INTEGER NOT NULL,
    version_catalogo INTEGER NOT NULL,
    -- `PartidaGuardada` en JSON comprimido; NULL si no se tuvo entera.
    registro         BLOB
);

CREATE INDEX partidas_terminada ON partidas(terminada DESC);

-- Qué deck jugó en cada asiento (para pedir las partidas de un deck).
CREATE TABLE partida_decks (
    partida_id TEXT NOT NULL REFERENCES partidas(id) ON DELETE CASCADE,
    jugador    INTEGER NOT NULL,
    deck_id    TEXT NOT NULL,
    puesto     INTEGER NOT NULL,
    PRIMARY KEY (partida_id, jugador)
);

CREATE INDEX partida_decks_deck ON partida_decks(deck_id);
