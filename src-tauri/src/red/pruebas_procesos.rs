//! La partida en red entre procesos de verdad: el binario de pruebas se
//! relanza a sí mismo como anfitrión y como invitados. Al anfitrión se le
//! mata (`SIGKILL`), sin el cierre limpio de QUIC que sí hacen las pruebas de
//! `pruebas.rs`, o se va por su pie (`Anfitrion::irse`). Mide cuánto tarda un
//! invitado en darse cuenta y seguir en cada caso.
//!
//! Va con `#[ignore]` porque tarda y lanza procesos:
//! `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --ignored --nocapture red::pruebas_procesos`

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use iroh::SecretKey;
use mesa::formato::{Formato, Variantes};
use mesa::sparring::Intensidad;
use mesa::{Accion, Azar, IdJugador};

use super::anfitrion::{atender, empezar_partida, Anfitrion};
use super::codigo::{secreto_nuevo, Codigo};
use super::invitado::{unirse, Union};
use super::mesa::{Mesa, MesaEnMemoria};
use super::nodo::{crear, Relay};
use super::protocolo::Rol;
use super::pruebas::{activo, epoch_invitado, es_anfitrion, esperar, huella, jugada, seq};
use crate::mesa::sesion::tests::mazo;
use crate::mesa::sesion::{ConfigMesa, JugadorMesa};

const JUGADORES: usize = 3;
/// La prueba que los hijos ejecutan para hacer de anfitrión o de invitado.
const NOMBRE_DEL_HIJO: &str = "red::pruebas_procesos::hijo";

/// Un proceso hijo que no se queda vivo si la prueba falla.
struct Hijo(Child);

impl Drop for Hijo {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn escribir(dir: &Path, nombre: &str, texto: &str) {
    let tmp = dir.join(format!("{nombre}.tmp"));
    std::fs::write(&tmp, texto).unwrap();
    std::fs::rename(&tmp, dir.join(nombre)).unwrap();
}

fn leer(dir: &Path, nombre: &str) -> Option<String> {
    std::fs::read_to_string(dir.join(nombre)).ok()
}

/// Lo que un hijo cuenta de su mesa: `seq huella es_anfitrión epoch activo`.
#[derive(Debug, Clone, PartialEq)]
struct Estado {
    seq: u64,
    huella: String,
    anfitrion: bool,
    epoch: Option<u32>,
    activo: bool,
}

fn estado(dir: &Path, rol: &str) -> Option<Estado> {
    let t = leer(dir, &format!("{rol}.estado"))?;
    let mut p = t.split(' ');
    Some(Estado {
        seq: p.next()?.parse().ok()?,
        huella: p.next()?.to_string(),
        anfitrion: p.next()? == "1",
        epoch: p.next()?.parse().ok(),
        activo: p.next()? == "1",
    })
}

fn publicar(dir: &Path, rol: &str, m: &MesaEnMemoria) {
    let (Some(s), Some(h)) = (seq(m), huella(m)) else {
        return;
    };
    let epoch = epoch_invitado(m).unwrap_or(0);
    escribir(
        dir,
        &format!("{rol}.estado"),
        &format!(
            "{s} {h} {} {epoch} {}",
            u8::from(es_anfitrion(m)),
            u8::from(activo(m))
        ),
    );
}

/// Juega al azar y cuenta su estado hasta que el padre crea `parar`.
async fn jugar(dir: &Path, rol: &str, m: &MesaEnMemoria, yo: IdJugador, jugadas: bool) {
    let mut azar = Azar::new(7 + u64::from(yo.0));
    let fin = Instant::now() + Duration::from_secs(240);
    while Instant::now() < fin {
        if leer(dir, "parar").is_some() {
            publicar(dir, rol, m);
            // Un rato más contando, para que el padre vea la misma partida.
            for _ in 0..100 {
                publicar(dir, rol, m);
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            return;
        }
        if jugadas && leer(dir, "jugar").is_some() {
            jugada(m, yo, &mut azar);
        }
        publicar(dir, rol, m);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn quedarse(m: &MesaEnMemoria) {
    m.con(|s| {
        let _ = s
            .as_mut()
            .unwrap()
            .accion(Accion::QuedarseCon { al_fondo: vec![] }, None);
    });
}

async fn como_anfitrion(dir: &Path) {
    let mesa = MesaEnMemoria::default();
    let ep = crear(SecretKey::generate(), Relay::Ninguno).await.unwrap();
    let anf = Anfitrion::nuevo("Host".into(), secreto_nuevo());
    *anf.deck.lock().unwrap() = Some("Prueba".into());
    tokio::spawn(atender(ep.clone(), anf.clone(), mesa.clone()));
    // El padre pide que se vaya a propósito (en vez de matarlo).
    let (dir2, anf2) = (dir.to_path_buf(), anf.clone());
    tokio::spawn(async move {
        loop {
            if leer(&dir2, "irse").is_some() {
                let cedida = anf2.irse().await;
                escribir(&dir2, "ido", if cedida { "cedida" } else { "cerrada" });
                std::process::exit(0);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });
    let codigo = Codigo {
        addr: ep.addr(),
        secreto: anf.secreto().to_string(),
    };
    escribir(dir, "codigo", &codigo.a_texto().unwrap());
    // Los invitados llegan de uno en uno (el asiento es el orden de llegada).
    loop {
        let n = anf.para_empezar().len();
        escribir(dir, "sala", &n.to_string());
        if n == JUGADORES - 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let config = ConfigMesa {
        jugadores: vec![JugadorMesa {
            deck_id: "d".into(),
            nombre: "Host".into(),
            color: "#e11d48".into(),
            lista: None,
            equipo: None,
        }],
        formato: Formato::Constructed,
        variantes: Variantes::default(),
        opciones: None,
        intensidad: Intensidad::Apagado,
        equipos: Vec::new(),
        al_mejor_de: None,
        primero: None,
    };
    empezar_partida(&anf, &mesa, ep.id(), config, mazo(), &[]).unwrap();
    quedarse(&mesa);
    jugar(dir, "host", &mesa, IdJugador(0), true).await;
}

async fn como_invitado(dir: &Path, i: usize) {
    let mesa = MesaEnMemoria::default();
    let ep = crear(SecretKey::generate(), Relay::Ninguno).await.unwrap();
    let codigo = loop {
        if let Some(t) = leer(dir, "codigo") {
            break Codigo::de_texto(&t).unwrap();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    let _conexion = unirse(
        ep,
        Union {
            codigo,
            apodo: format!("Invitado {i}"),
            rol: Rol::Jugador,
            deck: Some(mazo()),
        },
        mesa.clone(),
    )
    .await
    .unwrap();
    esperar("la bienvenida", || seq(&mesa).is_some()).await;
    quedarse(&mesa);
    jugar(dir, &format!("inv{i}"), &mesa, IdJugador(i as u8 + 1), true).await;
}

fn lanzar(dir: &Path, rol: &str, i: usize) -> Hijo {
    let hijo = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", NOMBRE_DEL_HIJO, "--ignored", "--nocapture"])
        .env("PB_PRUEBA_ROL", rol)
        .env("PB_PRUEBA_N", i.to_string())
        .env("PB_PRUEBA_DIR", dir)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    Hijo(hijo)
}

async fn hasta(que: &str, tope: Duration, cond: impl Fn() -> bool) -> Duration {
    let t0 = Instant::now();
    while t0.elapsed() < tope {
        if cond() {
            return t0.elapsed();
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("no llegó en {tope:?}: {que}");
}

/// No es una prueba: es lo que ejecuta cada proceso hijo. Sin `PB_PRUEBA_ROL`
/// (la lanzan a mano con `--ignored`), no hace nada.
#[tokio::test]
#[ignore = "lo lanzan las pruebas con procesos"]
async fn hijo() {
    let Ok(rol) = std::env::var("PB_PRUEBA_ROL") else {
        return;
    };
    let dir = PathBuf::from(std::env::var("PB_PRUEBA_DIR").unwrap());
    let i: usize = std::env::var("PB_PRUEBA_N").unwrap().parse().unwrap();
    match rol.as_str() {
        "host" => como_anfitrion(&dir).await,
        _ => como_invitado(&dir, i).await,
    }
}

/// Cómo se va el anfitrión.
#[derive(Clone, Copy)]
enum Salida {
    /// Se le mata (`SIGKILL`): no avisa, se nota al vencer el tiempo de QUIC.
    Matado,
    /// Sale por su pie (`Anfitrion::irse`): avisa de quién sigue.
    APorSuPie,
}

/// Tres procesos; el anfitrión se va y se mide cuánto tarda en seguir la partida.
/// Devuelve el tiempo hasta que el nuevo anfitrión deja jugar.
async fn escenario(salida: Salida) -> Duration {
    let dir = std::env::temp_dir().join(format!(
        "pb-procesos-{}-{}",
        std::process::id(),
        match salida {
            Salida::Matado => "kill",
            Salida::APorSuPie => "irse",
        }
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let largo = Duration::from_secs(90);

    let mut host = lanzar(&dir, "host", 0);
    hasta("el código del anfitrión", largo, || {
        leer(&dir, "codigo").is_some()
    })
    .await;
    let mut invitados = Vec::new();
    for i in 1..JUGADORES {
        invitados.push(lanzar(&dir, "inv", i));
        hasta("que el invitado esté en la sala", largo, || {
            leer(&dir, "sala").is_some_and(|n| n.trim() == i.to_string())
        })
        .await;
    }
    hasta("la partida empezada en los tres", largo, || {
        ["host", "inv1", "inv2"]
            .iter()
            .all(|r| estado(&dir, r).is_some())
    })
    .await;

    // 1. Movimiento en vivo con el anfitrión de verdad.
    escribir(&dir, "jugar", "1");
    tokio::time::sleep(Duration::from_secs(5)).await;
    let antes = estado(&dir, "inv1").unwrap().seq;
    assert!(antes > 20, "casi no se ha jugado: {antes}");

    // 2. El anfitrión se va.
    let t0 = Instant::now();
    match salida {
        Salida::Matado => {
            host.0.kill().unwrap();
            host.0.wait().unwrap();
        }
        Salida::APorSuPie => {
            escribir(&dir, "irse", "1");
            hasta("que el anfitrión ceda", largo, || {
                leer(&dir, "ido").is_some()
            })
            .await;
            assert_eq!(leer(&dir, "ido").as_deref(), Some("cedida"));
            host.0.wait().unwrap();
        }
    }

    // 3. Alguien recoge el testigo: uno es anfitrión y el otro lo sigue.
    let quien = |dir: &Path| {
        let a = estado(dir, "inv1")?;
        let b = estado(dir, "inv2")?;
        Some((a, b))
    };
    hasta(
        "un invitado hecho anfitrión y el otro con su época 2",
        Duration::from_secs(150),
        || {
            quien(&dir).is_some_and(|(a, b)| {
                (a.anfitrion && b.epoch == Some(2)) || (b.anfitrion && a.epoch == Some(2))
            })
        },
    )
    .await;
    eprintln!("nuevo anfitrión y seguidor a los {:?}", t0.elapsed());
    hasta("que el nuevo anfitrión deje jugar", largo, || {
        quien(&dir).is_some_and(|(a, b)| (a.anfitrion && a.activo) || (b.anfitrion && b.activo))
    })
    .await;
    let hasta_jugar = t0.elapsed();
    eprintln!("deja jugar a los {hasta_jugar:?}");

    // 4. Se sigue jugando entre los dos y acaban con la misma partida.
    let (a, b) = quien(&dir).unwrap();
    let despues = a.seq.max(b.seq);
    tokio::time::sleep(Duration::from_secs(5)).await;
    let (a, b) = quien(&dir).unwrap();
    assert!(
        a.seq.max(b.seq) > despues + 20,
        "no se juega tras el traspaso: {despues} → {}",
        a.seq.max(b.seq)
    );
    escribir(&dir, "parar", "1");
    hasta("la misma partida en los dos", largo, || {
        quien(&dir).is_some_and(|(a, b)| a.seq == b.seq && a.huella == b.huella)
    })
    .await;
    drop(invitados);
    let _ = std::fs::remove_dir_all(&dir);
    hasta_jugar
}

#[tokio::test]
#[ignore = "lanza procesos y tarda: se pide con --ignored"]
async fn procesos_reales_matar_al_anfitrion() {
    let t = escenario(Salida::Matado).await;
    eprintln!("traspaso tras SIGKILL: {t:?}");
}

#[tokio::test]
#[ignore = "lanza procesos y tarda: se pide con --ignored"]
async fn procesos_reales_el_anfitrion_se_va_a_proposito() {
    let t = escenario(Salida::APorSuPie).await;
    eprintln!("traspaso voluntario: {t:?}");
    // Sin esperar a que se note que falta: mucho menos que los ~30 s de QUIC.
    assert!(t < Duration::from_secs(15), "tardó {t:?}");
}
