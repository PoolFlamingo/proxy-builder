-- El idioma del deck elegido a mano (NULL: el que se calcula de las cartas).
ALTER TABLE decks ADD COLUMN lang_override TEXT;
