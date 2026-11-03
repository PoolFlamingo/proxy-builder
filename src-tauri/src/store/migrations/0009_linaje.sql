-- El linaje de cada deck: saber que dos decks de equipos distintos son «el
-- mismo» (pasar decks entre equipos, `pasar/`). Cada importación crea ids
-- nuevos a propósito, así que no sirve comparar ids ni nombres.
--
-- Un deck nuevo toma su propio id como linaje; uno duplicado, el suyo (es otro
-- deck); uno importado o recibido conserva el del que vino. `actualizado` es
-- la hora (ms) en que se cambió por última vez el contenido de esa versión del
-- deck, en el equipo donde se cambió: sirve para decir cuál de dos copias es
-- más reciente. Va aparte del deck, como las etiquetas y las listas: no toca
-- `updated_at`.
CREATE TABLE deck_linaje (
    deck_id     TEXT PRIMARY KEY REFERENCES decks(id) ON DELETE CASCADE,
    linaje      TEXT NOT NULL,
    actualizado INTEGER NOT NULL
);
CREATE INDEX deck_linaje_linaje ON deck_linaje(linaje);

-- Los decks que ya hay: cada uno su propio linaje.
INSERT INTO deck_linaje (deck_id, linaje, actualizado)
    SELECT id, id, updated_at FROM decks;
