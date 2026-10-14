-- Referencias a imágenes que no son ni el frente ni el dorso de una carta:
-- la ilustración y el emblema de un diseño, la imagen de «antes» de un
-- diseño y el emblema de una colección (la 2.0, editor de cartas). La
-- limpieza de huérfanas (`BlobStore::gc`) nunca borra una imagen apuntada
-- aquí.
CREATE TABLE blob_refs (
    owner_kind TEXT NOT NULL,   -- 'deck' | 'preset'
    owner_id   TEXT NOT NULL,
    blob       TEXT NOT NULL,
    PRIMARY KEY (owner_kind, owner_id, blob)
);
CREATE INDEX blob_refs_blob ON blob_refs(blob);
