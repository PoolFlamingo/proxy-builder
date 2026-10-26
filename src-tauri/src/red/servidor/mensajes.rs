//! **Los mensajes del protocolo v1 con el servidor de salas** (Fase F), los
//! mismos que `server/src/protocolo.ts` (`server/docs/protocolo.md`). Un
//! objeto JSON por mensaje con su tipo en `t`; lo que manda la app lleva
//! `id` y la respuesta lo devuelve en `re`.
//!
//! Las pruebas de contrato comprueban los dos lados: `fixtures/al-servidor.json`
//! lo escribe esta app (lo lee el servidor) y los ejemplos del servidor
//! (`server/test/ejemplos/del-servidor.json`) los lee esta app.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const VERSION_PROTOCOLO: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Plataforma {
    Escritorio,
    Android,
    Ios,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, rename = "RolPlaygroup")]
pub enum Rol {
    Dueno,
    Admin,
    Miembro,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, rename = "ComoEnSala")]
pub enum Como {
    Jugador,
    Espectador,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, rename = "FaseSala")]
pub enum Fase {
    Espera,
    Juego,
    Terminada,
}

/// Lo que la app sabe de su sala, para volver a registrarla si el servidor
/// se reinició.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistroSala {
    pub creador: String,
    pub playgroup: Option<String>,
    pub formato: String,
    pub max_jugadores: u8,
    pub espectadores: bool,
    pub fase: Fase,
    pub epoch: u32,
    pub host: Option<String>,
    pub sucesion: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Medida {
    pub clave: String,
    pub rtt_ms: u32,
    pub directo: bool,
}

/// Lo que manda la app (sin el `id`, que pone `Peticion`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum AlServidor {
    Hola {
        version: u32,
        clave: String,
        apodo: String,
        plataforma: Plataforma,
        firma: String,
    },
    Ping,
    PgCrear {
        nombre: String,
    },
    PgListar,
    PgSalir {
        playgroup: String,
    },
    PgExpulsar {
        playgroup: String,
        clave: String,
    },
    PgRol {
        playgroup: String,
        clave: String,
        rol: Rol,
    },
    InvCrear {
        #[serde(skip_serializing_if = "Option::is_none")]
        playgroup: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        sala: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        caduca_horas: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        usos_max: Option<u32>,
    },
    InvRevocar {
        codigo: String,
    },
    InvUsar {
        codigo: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        como: Option<Como>,
    },
    SalaCrear {
        #[serde(skip_serializing_if = "Option::is_none")]
        playgroup: Option<String>,
        formato: String,
        max_jugadores: u8,
        espectadores: bool,
    },
    SalaUnirse {
        sala: String,
        como: Como,
        #[serde(skip_serializing_if = "Option::is_none")]
        registro: Option<RegistroSala>,
    },
    SalaSalir,
    SalaDireccion {
        direccion: String,
    },
    SalaDeck {
        deck: Option<String>,
    },
    SalaLatencias {
        fila: Vec<Medida>,
    },
    SalaEmpezar,
    SalaHostCaido {
        epoch: u32,
    },
    SalaTerminada,
}

/// Una petición con su `id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Peticion {
    pub id: u64,
    #[serde(flatten)]
    pub m: AlServidor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "MiembroPlaygroup")]
pub struct Miembro {
    pub clave: String,
    pub apodo: String,
    pub rol: Rol,
    pub en_linea: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SalaResumen {
    pub id: String,
    pub formato: String,
    pub fase: Fase,
    pub creador: String,
    pub jugadores: u32,
    pub max_jugadores: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Playgroup {
    pub id: String,
    pub nombre: String,
    pub rol: Rol,
    pub miembros: Vec<Miembro>,
    pub salas: Vec<SalaResumen>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "MiembroSala")]
pub struct EnSala {
    pub clave: String,
    pub apodo: String,
    pub como: Como,
    pub plataforma: Plataforma,
    pub en_linea: bool,
    /// Su `EndpointAddr` de iroh en JSON (opaco para el servidor).
    pub direccion: Option<String>,
    /// El nombre del deck con el que juega, si ya lo preparó.
    pub deck: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "InfoSala")]
pub struct Sala {
    pub id: String,
    pub playgroup: Option<String>,
    pub formato: String,
    pub fase: Fase,
    pub creador: String,
    pub max_jugadores: u32,
    pub espectadores: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EstadoSala {
    pub sala: Sala,
    pub miembros: Vec<EnSala>,
    pub host: Option<String>,
    pub epoch: u32,
    pub sucesion: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum MotivoFuera {
    Terminada,
    Salida,
}

/// Lo que manda el servidor (el `re`, si lo lleva, va en `Llegada`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum DelServidor {
    Reto {
        nonce: String,
    },
    Bienvenido {
        servidor: String,
        clave: String,
    },
    Error {
        code: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        params: Option<BTreeMap<String, String>>,
    },
    Ok,
    Pong,
    PgLista {
        playgroups: Vec<Playgroup>,
    },
    Inv {
        codigo: String,
        caduca: Option<i64>,
        playgroup: Option<String>,
        sala: Option<String>,
    },
    SalaEstado(EstadoSala),
    SalaHost {
        sala: String,
        epoch: u32,
        host: String,
        sucesion: Vec<String>,
    },
    SalaFuera {
        sala: String,
        motivo: MotivoFuera,
    },
}

/// Un mensaje del servidor con su `re` (si responde a una petición).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Llegada {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub re: Option<u64>,
    #[serde(flatten)]
    pub m: DelServidor,
}

/// El texto que se firma en `hola`.
pub fn texto_a_firmar(nonce: &str) -> String {
    format!("proxy-builder/v1/{nonce}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const SALA: &str = "3f2b8c1e-5d7a-4c2e-9f10-2a6b7c8d9e01";
    const PG: &str = "9a8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d";

    /// Un ejemplo de cada mensaje que manda la app.
    fn ejemplos() -> Vec<Peticion> {
        let direccion = format!(r#"{{"id":"{A}","addrs":[]}}"#);
        let ms = vec![
            AlServidor::Hola {
                version: VERSION_PROTOCOLO,
                clave: A.into(),
                apodo: "Ana".into(),
                plataforma: Plataforma::Escritorio,
                firma: "0".repeat(128),
            },
            AlServidor::Ping,
            AlServidor::PgCrear {
                nombre: "Los del jueves".into(),
            },
            AlServidor::PgListar,
            AlServidor::PgSalir {
                playgroup: PG.into(),
            },
            AlServidor::PgExpulsar {
                playgroup: PG.into(),
                clave: B.into(),
            },
            AlServidor::PgRol {
                playgroup: PG.into(),
                clave: B.into(),
                rol: Rol::Admin,
            },
            AlServidor::InvCrear {
                playgroup: Some(PG.into()),
                sala: None,
                caduca_horas: Some(168),
                usos_max: Some(5),
            },
            AlServidor::InvRevocar {
                codigo: "7K3QX-9MZ2D".into(),
            },
            AlServidor::InvUsar {
                codigo: "7K3QX9MZ2D".into(),
                como: Some(Como::Jugador),
            },
            AlServidor::SalaCrear {
                playgroup: Some(PG.into()),
                formato: "commander".into(),
                max_jugadores: 4,
                espectadores: true,
            },
            AlServidor::SalaUnirse {
                sala: SALA.into(),
                como: Como::Jugador,
                registro: Some(RegistroSala {
                    creador: A.into(),
                    playgroup: Some(PG.into()),
                    formato: "commander".into(),
                    max_jugadores: 4,
                    espectadores: true,
                    fase: Fase::Juego,
                    epoch: 3,
                    host: Some(C.into()),
                    sucesion: vec![C.into(), A.into(), B.into()],
                }),
            },
            AlServidor::SalaSalir,
            AlServidor::SalaDireccion { direccion },
            AlServidor::SalaDeck {
                deck: Some("Atraxa superfriends".into()),
            },
            AlServidor::SalaLatencias {
                fila: vec![
                    Medida {
                        clave: B.into(),
                        rtt_ms: 23,
                        directo: true,
                    },
                    Medida {
                        clave: C.into(),
                        rtt_ms: 71,
                        directo: false,
                    },
                ],
            },
            AlServidor::SalaEmpezar,
            AlServidor::SalaHostCaido { epoch: 1 },
            AlServidor::SalaTerminada,
        ];
        ms.into_iter()
            .enumerate()
            .map(|(i, m)| Peticion {
                id: i as u64 + 1,
                m,
            })
            .collect()
    }

    fn fixtures() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/red/servidor/fixtures")
    }

    /// Lo que manda la app es lo de `fixtures/al-servidor.json` (con
    /// `PB_FIXTURES=1` se reescribe).
    #[test]
    fn los_mensajes_de_la_app_son_los_de_los_fixtures() {
        let v = serde_json::to_value(ejemplos()).unwrap();
        let ruta = fixtures().join("al-servidor.json");
        if std::env::var("PB_FIXTURES").is_ok() {
            std::fs::create_dir_all(fixtures()).unwrap();
            let mut t = serde_json::to_string_pretty(&v).unwrap();
            t.push('\n');
            std::fs::write(&ruta, t.replace("  ", "\t")).unwrap();
        }
        let guardado: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&ruta).unwrap()).unwrap();
        assert_eq!(v, guardado, "rehaz los fixtures con PB_FIXTURES=1");
        // Y se leen de vuelta igual.
        let de_vuelta: Vec<Peticion> = serde_json::from_value(guardado).unwrap();
        assert_eq!(de_vuelta, ejemplos());
    }

    /// Lo que manda el servidor (sus ejemplos, si el servidor está clonado
    /// dentro de la app) lo entiende la app.
    #[test]
    fn los_ejemplos_del_servidor_se_entienden() {
        let ruta = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../server/test/ejemplos/del-servidor.json");
        let Ok(texto) = std::fs::read_to_string(&ruta) else {
            eprintln!("aviso: sin el servidor en server/, no se comprueban sus ejemplos");
            return;
        };
        let v: Vec<serde_json::Value> = serde_json::from_str(&texto).unwrap();
        let mut tipos = std::collections::BTreeSet::new();
        for m in v {
            let l: Llegada =
                serde_json::from_value(m.clone()).unwrap_or_else(|e| panic!("{m}: {e}"));
            tipos.insert(m["t"].as_str().unwrap_or_default().to_string());
            // Y escrito de nuevo es lo mismo (no se pierde ningún campo).
            assert_eq!(serde_json::to_value(&l).unwrap(), m);
        }
        assert_eq!(tipos.len(), 10, "{tipos:?}");
    }
}
