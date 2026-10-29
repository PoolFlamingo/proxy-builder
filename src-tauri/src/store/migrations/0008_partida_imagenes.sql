-- Las imágenes propias que pinta la repetición de cada partida del historial
-- (las de todos los jugadores, por su huella). La copia vive en
-- `app_data_dir/partidas/imagenes/`: la caché de la mesa se puede borrar y
-- las de los demás no se pueden volver a preparar. Al borrar la partida, la
-- copia que ya no use ninguna otra se borra.
CREATE TABLE partida_imagenes (
    partida_id TEXT NOT NULL REFERENCES partidas(id) ON DELETE CASCADE,
    sha        TEXT NOT NULL,
    PRIMARY KEY (partida_id, sha)
);
CREATE INDEX partida_imagenes_sha ON partida_imagenes(sha);
