//! **El historial de partidas** (`partidas` y `partida_decks`, migración
//! 0006): cada partida terminada con su resultado y, si quien la guarda la
//! tuvo entera, su registro comprimido para repetirla. Aparte del deck: jugar
//! no cambia su `updated_at`, y al borrar un deck sus partidas se quedan.

use mesa::formato::Formato;
use mesa::modelo::ComoGano;
use mesa::{IdJugador, VERSION_CATALOGO};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::db::Db;
use super::listas;
use crate::error::{AppError, AppResult, ErrorCode};

/// Cómo se jugó.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ModoPartida {
    /// Un jugador (con o sin fantasma).
    Solitario,
    /// Varios en este mismo equipo.
    Local,
    /// Por la red.
    EnLinea,
}

/// Quién jugó cada asiento y cómo acabó.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct JugadorHistorial {
    pub jugador: IdJugador,
    pub nombre: String,
    pub color: String,
    /// El deck de la biblioteca (vacío si no se sabe: un invitado en red).
    pub deck_id: String,
    /// Cómo se llamaba el deck entonces.
    pub deck: String,
    /// La lista para jugar que se llevó (su id; vacío si fue el deck entero).
    #[serde(default)]
    pub lista_id: String,
    /// Cómo se llamaba la lista entonces (vacío si fue el deck entero).
    #[serde(default)]
    pub lista: String,
    /// Cuántas cartas tenía la lista, comandantes incluidos (0 si fue el deck
    /// entero).
    #[serde(default)]
    pub cartas_lista: u32,
    pub puesto: u32,
}

/// Una partida para guardar.
#[derive(Debug, Clone)]
pub struct NuevaPartida {
    pub id: String,
    pub empezada: i64,
    pub terminada: i64,
    pub formato: Formato,
    pub modo: ModoPartida,
    pub jugadores: Vec<JugadorHistorial>,
    pub ganadores: Vec<IdJugador>,
    pub como: ComoGano,
    pub nota: String,
    pub rondas: u32,
    pub duracion_ms: u64,
    pub registro: Option<Vec<u8>>,
    /// Las huellas de las imágenes propias que pinta su repetición (vacío sin
    /// registro): se copian aparte para que la repetición no dependa de la caché.
    pub imagenes: Vec<String>,
}

/// Una fila de la lista de un deck.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FilaPartida {
    pub id: String,
    #[ts(type = "number")]
    pub terminada: i64,
    pub formato: Formato,
    pub modo: ModoPartida,
    pub jugadores: Vec<JugadorHistorial>,
    pub ganadores: Vec<IdJugador>,
    pub como: ComoGano,
    pub nota: String,
    pub rondas: u32,
    #[ts(type = "number")]
    pub duracion_ms: u64,
    /// El mejor puesto de este deck en la partida.
    pub puesto: u32,
    pub gano: bool,
    /// Se puede repetir (se guardó entera y esta versión la entiende).
    pub repetible: bool,
}

/// Las partidas de un deck y cómo le ha ido.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HistorialDeck {
    pub partidas: u32,
    pub victorias: u32,
    pub filas: Vec<FilaPartida>,
}

/// Todas las partidas, de la más reciente a la más antigua (una página).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HistorialGeneral {
    /// Cuántas partidas hay en total (no solo en esta página).
    pub total: u32,
    /// En cuántas ganó alguno de tus decks.
    pub victorias: u32,
    pub filas: Vec<FilaPartida>,
}

fn json<T: Serialize>(v: &T) -> AppResult<String> {
    serde_json::to_string(v).map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))
}

/// Guarda una partida terminada (o la reemplaza si se vuelve a terminar).
pub fn guardar(db: &Db, p: &NuevaPartida) -> AppResult<()> {
    let mut p = p.clone();
    completar_listas(db, &mut p);
    let p = &p;
    let mut c = db.conn()?;
    let tx = c.transaction()?;
    tx.execute("DELETE FROM partidas WHERE id = ?1", [&p.id])?;
    tx.execute(
        "INSERT INTO partidas (id, empezada, terminada, formato, modo, jugadores, ganadores,
                               como, nota, rondas, duracion_ms, version_catalogo, registro)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            p.id,
            p.empezada,
            p.terminada,
            json(&p.formato)?,
            json(&p.modo)?,
            json(&p.jugadores)?,
            json(&p.ganadores)?,
            json(&p.como)?,
            p.nota,
            p.rondas,
            i64::try_from(p.duracion_ms).unwrap_or(i64::MAX),
            VERSION_CATALOGO,
            p.registro,
        ],
    )?;
    for sha in &p.imagenes {
        tx.execute(
            "INSERT OR IGNORE INTO partida_imagenes (partida_id, sha) VALUES (?1, ?2)",
            params![p.id, sha],
        )?;
    }
    for j in p.jugadores.iter().filter(|j| !j.deck_id.is_empty()) {
        tx.execute(
            "INSERT INTO partida_decks (partida_id, jugador, deck_id, puesto)
             VALUES (?1, ?2, ?3, ?4)",
            params![p.id, j.jugador.0, j.deck_id, j.puesto],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Apunta cómo se llama cada lista para jugar y cuántas cartas tiene *ahora*,
/// para que el historial lo enseñe aunque luego se renombre o se borre. Si la
/// lista ya no existe o no se puede leer, se guarda solo su id.
fn completar_listas(db: &Db, p: &mut NuevaPartida) {
    for j in p.jugadores.iter_mut().filter(|j| !j.lista_id.is_empty()) {
        if !j.lista.is_empty() {
            continue;
        }
        if let Ok(Some(l)) = listas::una(db, &j.deck_id, &j.lista_id) {
            j.cartas_lista = (l.comandantes.len() + l.cartas.len()) as u32;
            j.lista = l.nombre;
        }
    }
}

fn de_json<T: for<'a> Deserialize<'a>>(s: &str) -> AppResult<T> {
    serde_json::from_str(s).map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))
}

/// Una fila de `partidas` (con su mejor puesto y si se puede repetir, en las
/// columnas 10 y 11) como `FilaPartida`.
fn fila_de(r: &rusqlite::Row<'_>) -> rusqlite::Result<FilaPartida> {
    let sql = |e: AppError| rusqlite::Error::ToSqlConversionFailure(Box::new(e));
    let texto = |i: usize| r.get::<_, String>(i);
    let puesto: u32 = r.get(10)?;
    Ok(FilaPartida {
        id: r.get(0)?,
        terminada: r.get(1)?,
        formato: de_json(&texto(2)?).map_err(sql)?,
        modo: de_json(&texto(3)?).map_err(sql)?,
        jugadores: de_json(&texto(4)?).map_err(sql)?,
        ganadores: de_json(&texto(5)?).map_err(sql)?,
        como: de_json(&texto(6)?).map_err(sql)?,
        nota: r.get(7)?,
        rondas: r.get(8)?,
        duracion_ms: u64::try_from(r.get::<_, i64>(9)?).unwrap_or(0),
        puesto,
        gano: puesto == 1,
        repetible: r.get(11)?,
    })
}

/// Las partidas de un deck, de la más reciente a la más antigua. Cada partida
/// cuenta una vez aunque el deck se haya jugado en dos asientos (vale su
/// mejor puesto).
pub fn de_deck(db: &Db, deck_id: &str) -> AppResult<HistorialDeck> {
    let c = db.conn()?;
    let mut st = c.prepare(
        "SELECT p.id, p.terminada, p.formato, p.modo, p.jugadores, p.ganadores, p.como, p.nota,
                p.rondas, p.duracion_ms, MIN(d.puesto),
                (p.registro IS NOT NULL AND p.version_catalogo <= ?2)
         FROM partidas p JOIN partida_decks d ON d.partida_id = p.id
         WHERE d.deck_id = ?1
         GROUP BY p.id
         ORDER BY p.terminada DESC, p.id",
    )?;
    let filas: Vec<FilaPartida> = st
        .query_map(params![deck_id, VERSION_CATALOGO], fila_de)?
        .collect::<Result<_, _>>()?;
    Ok(HistorialDeck {
        partidas: filas.len() as u32,
        victorias: filas.iter().filter(|f| f.gano).count() as u32,
        filas,
    })
}

/// Más partidas que esto no se piden de una vez.
pub const MAX_PAGINA: u32 = 500;

/// Todas las partidas terminadas, de todos los decks, de la más reciente a la
/// más antigua: `limite` a partir de `desde` (como mucho `MAX_PAGINA`). El
/// puesto de cada una es el mejor de los asientos con un deck de la
/// biblioteca (0 si no hubo ninguno: un invitado en red).
pub fn todas(db: &Db, limite: u32, desde: u32) -> AppResult<HistorialGeneral> {
    let c = db.conn()?;
    let mut st = c.prepare(
        "SELECT p.id, p.terminada, p.formato, p.modo, p.jugadores, p.ganadores, p.como, p.nota,
                p.rondas, p.duracion_ms, COALESCE(MIN(d.puesto), 0),
                (p.registro IS NOT NULL AND p.version_catalogo <= ?1)
         FROM partidas p LEFT JOIN partida_decks d ON d.partida_id = p.id
         GROUP BY p.id
         ORDER BY p.terminada DESC, p.id
         LIMIT ?2 OFFSET ?3",
    )?;
    let filas: Vec<FilaPartida> = st
        .query_map(
            params![VERSION_CATALOGO, limite.min(MAX_PAGINA), desde],
            fila_de,
        )?
        .collect::<Result<_, _>>()?;
    let total: u32 = c.query_row("SELECT COUNT(*) FROM partidas", [], |r| r.get(0))?;
    let victorias: u32 = c.query_row(
        "SELECT COUNT(DISTINCT partida_id) FROM partida_decks WHERE puesto = 1",
        [],
        |r| r.get(0),
    )?;
    Ok(HistorialGeneral {
        total,
        victorias,
        filas,
    })
}

/// El registro comprimido de una partida, si se guardó.
pub fn registro(db: &Db, id: &str) -> AppResult<Option<Vec<u8>>> {
    Ok(db
        .conn()?
        .query_row("SELECT registro FROM partidas WHERE id = ?1", [id], |r| {
            r.get::<_, Option<Vec<u8>>>(0)
        })
        .optional()?
        .flatten())
}

/// Borra una partida. Devuelve las huellas de las imágenes guardadas para su
/// repetición que ya no usa ninguna otra (para borrar sus copias).
pub fn borrar(db: &Db, id: &str) -> AppResult<Vec<String>> {
    let mut c = db.conn()?;
    let tx = c.transaction()?;
    let suyas: Vec<String> = {
        let mut st = tx.prepare("SELECT sha FROM partida_imagenes WHERE partida_id = ?1")?;
        let v = st
            .query_map([id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        v
    };
    tx.execute("DELETE FROM partidas WHERE id = ?1", [id])?;
    let mut sueltas = Vec::new();
    for sha in suyas {
        let usada: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM partida_imagenes WHERE sha = ?1)",
            [&sha],
            |r| r.get(0),
        )?;
        if !usada {
            sueltas.push(sha);
        }
    }
    tx.commit()?;
    Ok(sueltas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::decks;

    fn jugador(i: u8, deck_id: &str, puesto: u32) -> JugadorHistorial {
        JugadorHistorial {
            jugador: IdJugador(i),
            nombre: format!("J{i}"),
            color: "#fff".into(),
            deck_id: deck_id.into(),
            deck: format!("Deck {deck_id}"),
            lista_id: String::new(),
            lista: String::new(),
            cartas_lista: 0,
            puesto,
        }
    }

    fn partida(
        id: &str,
        t: i64,
        js: Vec<JugadorHistorial>,
        registro: Option<Vec<u8>>,
    ) -> NuevaPartida {
        NuevaPartida {
            id: id.into(),
            empezada: t - 1_000,
            terminada: t,
            formato: Formato::Commander,
            modo: ModoPartida::Local,
            ganadores: js
                .iter()
                .filter(|j| j.puesto == 1)
                .map(|j| j.jugador)
                .collect(),
            jugadores: js,
            como: ComoGano::Combo,
            nota: "gg".into(),
            rondas: 7,
            duracion_ms: 1_000,
            registro,
            imagenes: Vec::new(),
        }
    }

    #[test]
    fn al_borrar_una_partida_solo_quedan_sueltas_las_imagenes_que_nadie_usa() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("t.db")).unwrap();
        let sha = |c: char| c.to_string().repeat(64);
        let mut a = partida("a", 10, vec![jugador(0, "", 1)], Some(vec![1]));
        a.imagenes = vec![sha('a'), sha('b')];
        let mut b = partida("b", 20, vec![jugador(0, "", 1)], Some(vec![1]));
        b.imagenes = vec![sha('b')];
        guardar(&db, &a).unwrap();
        guardar(&db, &b).unwrap();
        assert_eq!(
            borrar(&db, "a").unwrap(),
            vec![sha('a')],
            "la «b» la usa otra"
        );
        assert_eq!(borrar(&db, "b").unwrap(), vec![sha('b')]);
    }

    #[test]
    fn guardar_y_leer_las_partidas_de_un_deck() {
        let db = Db::in_memory().unwrap();
        guardar(
            &db,
            &partida(
                "a",
                10,
                vec![jugador(0, "x", 1), jugador(1, "y", 2)],
                Some(vec![1, 2]),
            ),
        )
        .unwrap();
        guardar(
            &db,
            &partida("b", 20, vec![jugador(0, "x", 3), jugador(1, "y", 1)], None),
        )
        .unwrap();
        guardar(&db, &partida("c", 30, vec![jugador(0, "y", 1)], None)).unwrap();
        let h = de_deck(&db, "x").unwrap();
        assert_eq!((h.partidas, h.victorias), (2, 1));
        assert_eq!(
            h.filas.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
            vec!["b", "a"],
            "la más reciente primero"
        );
        assert!(h.filas[1].gano && h.filas[1].repetible);
        assert!(!h.filas[0].gano && !h.filas[0].repetible);
        assert_eq!(h.filas[1].jugadores.len(), 2);
        assert_eq!(h.filas[1].como, ComoGano::Combo);
        assert_eq!(registro(&db, "a").unwrap(), Some(vec![1, 2]));
        assert_eq!(registro(&db, "b").unwrap(), None);
        assert_eq!(registro(&db, "no").unwrap(), None);
        assert_eq!(de_deck(&db, "y").unwrap().partidas, 3);
        assert_eq!(de_deck(&db, "z").unwrap().partidas, 0);
    }

    #[test]
    fn un_deck_en_dos_asientos_cuenta_una_vez_con_su_mejor_puesto() {
        let db = Db::in_memory().unwrap();
        guardar(
            &db,
            &partida("a", 10, vec![jugador(0, "x", 2), jugador(1, "x", 1)], None),
        )
        .unwrap();
        let h = de_deck(&db, "x").unwrap();
        assert_eq!((h.partidas, h.victorias), (1, 1));
    }

    #[test]
    fn volver_a_terminar_reemplaza_y_los_invitados_sin_deck_no_cuelgan() {
        let db = Db::in_memory().unwrap();
        guardar(&db, &partida("a", 10, vec![jugador(0, "x", 2)], None)).unwrap();
        guardar(&db, &partida("a", 12, vec![jugador(0, "x", 1)], None)).unwrap();
        let h = de_deck(&db, "x").unwrap();
        assert_eq!((h.partidas, h.victorias), (1, 1));
        assert_eq!(h.filas[0].terminada, 12);
        guardar(&db, &partida("b", 13, vec![jugador(0, "", 1)], None)).unwrap();
        let n: i64 = db
            .conn()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM partida_decks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn borrar_la_partida_y_borrar_el_deck() {
        let db = Db::in_memory().unwrap();
        let d = decks::create(&db, "Grande", None).unwrap();
        guardar(&db, &partida("a", 10, vec![jugador(0, &d.id, 1)], None)).unwrap();
        guardar(&db, &partida("b", 20, vec![jugador(0, &d.id, 2)], None)).unwrap();
        // Al borrar el deck, sus partidas se quedan.
        decks::delete(&db, &d.id).unwrap();
        assert_eq!(de_deck(&db, &d.id).unwrap().partidas, 2);
        borrar(&db, "a").unwrap();
        let h = de_deck(&db, &d.id).unwrap();
        assert_eq!(h.partidas, 1);
        let filas_deck: i64 = db
            .conn()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM partida_decks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            filas_deck, 1,
            "la fila de partida_decks se va con su partida"
        );
    }

    #[test]
    fn jugar_no_cambia_la_fecha_del_deck() {
        let db = Db::in_memory().unwrap();
        let d = decks::create(&db, "Grande", None).unwrap();
        let fecha = |db: &Db| -> i64 {
            db.conn()
                .unwrap()
                .query_row("SELECT updated_at FROM decks WHERE id = ?1", [&d.id], |r| {
                    r.get(0)
                })
                .unwrap()
        };
        let antes = fecha(&db);
        guardar(&db, &partida("a", 10, vec![jugador(0, &d.id, 1)], None)).unwrap();
        de_deck(&db, &d.id).unwrap();
        assert_eq!(fecha(&db), antes);
    }

    #[test]
    fn una_base_vieja_con_la_5_pasa_a_la_6() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("library.db");
        {
            // Una base que llegó hasta la migración 5.
            let c = rusqlite::Connection::open(&ruta).unwrap();
            c.execute_batch(
                "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL);",
            )
            .unwrap();
            for (v, sql) in [
                (1, include_str!("migrations/0001_init.sql")),
                (2, include_str!("migrations/0002_blob_refs.sql")),
                (3, include_str!("migrations/0003_biblioteca.sql")),
                (4, include_str!("migrations/0004_idioma_deck.sql")),
                (5, include_str!("migrations/0005_listas_juego.sql")),
            ] {
                c.execute_batch(sql).unwrap();
                c.execute("INSERT INTO schema_migrations VALUES (?1, 0)", [v])
                    .unwrap();
            }
        }
        let db = Db::open(&ruta).unwrap();
        guardar(&db, &partida("a", 10, vec![jugador(0, "x", 1)], None)).unwrap();
        assert_eq!(de_deck(&db, "x").unwrap().partidas, 1);
    }

    #[test]
    fn todas_las_partidas_de_todos_los_decks_con_su_lista() {
        let db = Db::in_memory().unwrap();
        let d = decks::create(&db, "Grande", None).unwrap();
        let lista = listas::guardar(
            &db,
            &d.id,
            &mesa::lista::ListaDeJuego {
                id: String::new(),
                nombre: "Cien".into(),
                comandantes: vec!["c1".into()],
                cartas: vec!["a".into(), "b".into()],
            },
        )
        .unwrap();
        let mut con_lista = jugador(0, &d.id, 1);
        con_lista.lista_id = lista.id.clone();
        guardar(
            &db,
            &partida("a", 10, vec![con_lista, jugador(1, "y", 2)], None),
        )
        .unwrap();
        guardar(
            &db,
            &partida("b", 20, vec![jugador(0, "y", 3)], Some(vec![1])),
        )
        .unwrap();
        guardar(&db, &partida("c", 30, vec![jugador(0, "", 1)], None)).unwrap();

        let h = todas(&db, 50, 0).unwrap();
        assert_eq!(
            (h.total, h.victorias),
            (3, 1),
            "la de c no tiene deck propio"
        );
        assert_eq!(
            h.filas.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
            vec!["c", "b", "a"]
        );
        // El invitado sin deck: sin puesto ni victoria. La repetible, repetible.
        assert_eq!((h.filas[0].puesto, h.filas[0].gano), (0, false));
        assert!(h.filas[1].repetible && !h.filas[2].repetible);
        // La lista se apuntó con su nombre y su tamaño, y se queda aunque se
        // renombre o se borre después.
        let mia = &h.filas[2].jugadores[0];
        assert_eq!((mia.lista.as_str(), mia.cartas_lista), ("Cien", 3));
        assert_eq!(mia.lista_id, lista.id);
        listas::borrar(&db, &d.id, &lista.id).unwrap();
        assert_eq!(
            todas(&db, 50, 0).unwrap().filas[2].jugadores[0].lista,
            "Cien"
        );
        // El deck entero no lleva lista.
        assert_eq!(h.filas[1].jugadores[0].lista, "");
    }

    #[test]
    fn las_paginas_del_historial_general() {
        let db = Db::in_memory().unwrap();
        for i in 0..5 {
            guardar(
                &db,
                &partida(
                    &format!("p{i}"),
                    i64::from(i),
                    vec![jugador(0, "x", 1)],
                    None,
                ),
            )
            .unwrap();
        }
        let h = todas(&db, 2, 0).unwrap();
        assert_eq!((h.total, h.filas.len()), (5, 2));
        assert_eq!(h.filas[0].id, "p4");
        let h = todas(&db, 2, 4).unwrap();
        assert_eq!(
            h.filas.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
            vec!["p0"]
        );
        assert!(todas(&db, 2, 9).unwrap().filas.is_empty());
        assert_eq!(todas(&db, 10_000, 0).unwrap().filas.len(), 5);
    }

    #[test]
    fn una_partida_guardada_antes_de_las_listas_sigue_leyendose() {
        let db = Db::in_memory().unwrap();
        guardar(&db, &partida("a", 10, vec![jugador(0, "x", 1)], None)).unwrap();
        // Como la guardaba la fase H: sin los campos de la lista.
        db.conn()
            .unwrap()
            .execute(
                "UPDATE partidas SET jugadores = ?1",
                [r##"[{"jugador":0,"nombre":"J0","color":"#fff","deckId":"x","deck":"Deck x","puesto":1}]"##],
            )
            .unwrap();
        let h = todas(&db, 10, 0).unwrap();
        assert_eq!(h.filas[0].jugadores[0].lista, "");
        assert_eq!(de_deck(&db, "x").unwrap().partidas, 1);
    }
}
