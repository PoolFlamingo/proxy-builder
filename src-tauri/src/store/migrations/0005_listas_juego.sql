-- Las listas para jugar de un deck (mesa de juego): de un deck con más cartas
-- de las que se juegan (variantes de diseño, alternativas), las elegidas. Van
-- aparte del deck, como las etiquetas: el autoguardado del deck no las pisa.
CREATE TABLE deck_listas (
    deck_id     TEXT NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
    id          TEXT NOT NULL,
    nombre      TEXT NOT NULL,
    -- JSON: ids de copias del deck.
    comandantes TEXT NOT NULL,
    cartas      TEXT NOT NULL,
    updated_at  INTEGER NOT NULL,
    PRIMARY KEY (deck_id, id)
);
