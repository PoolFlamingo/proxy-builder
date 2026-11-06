//! **El protocolo de pasar decks** (`proxy-builder/deck/1`).
//!
//! Un stream bidireccional por intercambio. Tramas de `u32` (big endian) de
//! longitud + JSON, como el de la mesa (`red/protocolo.rs`), y después de un
//! `Paquete` van, sin trama, exactamente `tamano` bytes del `.pbdeck`.
//!
//! ```text
//! emisor                              receptor
//!   Hola { version, secreto?, pase?, apodo } →
//!                                     ← Listo { apodo }  |  Rechazo { motivo }
//!   Oferta { decks } →
//!                                     ← Respuesta { decisiones }
//!   Paquete { indice, tamano, huella } + bytes →      (por cada deck aceptado)
//!                                     ← Hecho { indice, ok, motivo? }
//!   Fin →
//!                                     ← Adios
//! ```
//!
//! Topes (lo que pasa se corta y la conexión se cierra): 64 KiB por trama
//! (también la oferta) y 32 decks por oferta. **Un paquete no tiene tope fijo**:
//! manda el espacio libre de quien recibe (`hay_espacio`), que además ha visto
//! el peso y ha aceptado.
//!
//! 🛑 Todo lo que llega de otro equipo es entrada no confiable: tramas con
//! tope antes de leerlas, JSON tipado, la oferta validada (`validar_oferta`) y
//! el paquete comprobado (tamaño y SHA-256 aquí, y `package::import` después).

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use super::tipos::{DeckOfrecido, Eleccion};
use crate::red::protocolo::{escribir_trama, leer_trama, ErrorTrama};

/// La versión del protocolo (la del ALPN).
pub const VERSION: u32 = 1;
/// Lo más grande que puede ser una trama de control, la oferta incluida.
pub const TOPE_TRAMA: usize = 64 * 1024;
pub const MAX_DECKS: usize = 32;
pub const MAX_NOMBRE: usize = 120;
pub const MAX_APODO: usize = 32;
/// Lo más largo que puede ser la portada en base64.
pub const MAX_PORTADA: usize = 6 * 1024;
const TROZO: usize = 64 * 1024;

/// De emisor a receptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum AlReceptor {
    Hola {
        version: u32,
        /// El del código en texto o el QR; sin él, hay que aceptar el
        /// encuentro a mano (el código de 8 cifras).
        #[serde(default)]
        secreto: Option<String>,
        /// El pase del anuncio de la red local (`encontrar::Anuncio`): dice que
        /// quien llama ha visto a este equipo en la lista de «cerca». Con él
        /// aún hay que aceptar el encuentro a mano.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pase: Option<String>,
        apodo: String,
    },
    Oferta {
        decks: Vec<DeckOfrecido>,
    },
    /// Va seguido de `tamano` bytes sin trama.
    Paquete {
        indice: u32,
        tamano: u64,
        /// SHA-256 del paquete entero, en hexadecimal.
        huella: String,
    },
    Fin,
}

/// De receptor a emisor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum AlEmisor {
    Listo {
        apodo: String,
    },
    /// No se acepta (`motivo`: `version`, `codigo`, `no_admite`, `ocupado`,
    /// `rechazado`, `caducado`).
    Rechazo {
        motivo: String,
    },
    /// Una elección por cada deck de la oferta, en el mismo orden.
    Respuesta {
        decisiones: Vec<Eleccion>,
    },
    Hecho {
        indice: u32,
        ok: bool,
        #[serde(default)]
        motivo: Option<String>,
    },
    Adios,
}

pub async fn escribir<W: AsyncWrite + Unpin, T: Serialize>(
    w: &mut W,
    m: &T,
) -> Result<(), ErrorTrama> {
    escribir_trama(w, &serde_json::to_vec(m)?, TOPE_TRAMA).await
}

pub async fn leer<R: AsyncRead + Unpin, T: DeserializeOwned>(r: &mut R) -> Result<T, ErrorTrama> {
    let b = leer_trama(r, TOPE_TRAMA).await?;
    Ok(serde_json::from_slice(&b)?)
}

/// ¿Es un texto de una línea sin caracteres de control?
fn texto_sano(t: &str, max: usize) -> bool {
    !t.trim().is_empty() && t.chars().count() <= max && !t.chars().any(char::is_control)
}

/// Comprueba la oferta de fuera: cuántos decks, nombres y linajes. El peso solo
/// se enseña: lo que cuenta es el del paquete (`recibir::recibir_uno`). No toca
/// la portada (la limpia quien la enseña, en Rust).
pub fn validar_oferta(decks: &[DeckOfrecido]) -> Result<(), &'static str> {
    if decks.is_empty() || decks.len() > MAX_DECKS {
        return Err("decks");
    }
    for d in decks {
        if !texto_sano(&d.nombre, MAX_NOMBRE) {
            return Err("nombre");
        }
        if d.cartas > 100_000 {
            return Err("cartas");
        }
        if d.linaje
            .as_deref()
            .is_some_and(|l| l.is_empty() || l.len() > 64 || !l.chars().all(es_de_id))
        {
            return Err("linaje");
        }
        if d.portada.as_deref().is_some_and(|p| p.len() > MAX_PORTADA) {
            return Err("portada");
        }
    }
    Ok(())
}

fn es_de_id(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// Un apodo de fuera, limpio (para enseñarlo): sin saltos ni caracteres de
/// control y con un largo razonable.
pub fn apodo_limpio(a: &str) -> String {
    let limpio: String = a
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    limpio.chars().take(MAX_APODO).collect()
}

/// Sin un solo byte en este tiempo, quien manda se da por perdido (un emisor
/// parado no deja «Recibir» ocupado sin fin). Es holgado: por datos móviles y
/// por un relé con tope de velocidad hay parones de muchos segundos; si la
/// conexión se muere de verdad, QUIC lo dice antes.
pub fn plazo_sin_bytes() -> std::time::Duration {
    #[cfg(test)]
    {
        // Las pruebas de velocidad por el relé necesitan el de verdad.
        if std::env::var_os("PB_PLAZO_REAL").is_none() {
            return std::time::Duration::from_millis(300);
        }
    }
    std::time::Duration::from_secs(120)
}

/// Copia exactamente `tamano` bytes de `r` a `w` por trozos, calculando su
/// SHA-256 y avisando del avance. Si el otro lado corta antes, es un error;
/// si pasa [`plazo_sin_bytes`] sin que llegue nada, un `Io` de `TimedOut`.
pub async fn recibir_bytes<R, W, F>(
    r: &mut R,
    w: &mut W,
    tamano: u64,
    mut avance: F,
) -> Result<String, ErrorTrama>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    F: FnMut(u64),
{
    let mut h = Sha256::new();
    let mut buf = vec![0u8; TROZO];
    let mut falta = tamano;
    let mut hecho = 0u64;
    while falta > 0 {
        let n = usize::try_from(falta.min(TROZO as u64)).unwrap_or(TROZO);
        let mut leidos = 0;
        while leidos < n {
            let m = tokio::time::timeout(plazo_sin_bytes(), r.read(&mut buf[leidos..n]))
                .await
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::TimedOut))??;
            if m == 0 {
                return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof).into());
            }
            leidos += m;
        }
        h.update(&buf[..n]);
        w.write_all(&buf[..n]).await?;
        falta -= n as u64;
        hecho += n as u64;
        avance(hecho);
    }
    w.flush().await?;
    Ok(hex(&h.finalize()))
}

/// Manda un fichero entero por trozos, avisando del avance.
pub async fn enviar_bytes<R, W, F>(
    r: &mut R,
    w: &mut W,
    tamano: u64,
    mut avance: F,
) -> Result<(), ErrorTrama>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    F: FnMut(u64),
{
    let mut buf = vec![0u8; TROZO];
    let mut falta = tamano;
    let mut hecho = 0u64;
    while falta > 0 {
        let n = usize::try_from(falta.min(TROZO as u64)).unwrap_or(TROZO);
        r.read_exact(&mut buf[..n]).await?;
        w.write_all(&buf[..n]).await?;
        falta -= n as u64;
        hecho += n as u64;
        avance(hecho);
    }
    w.flush().await?;
    Ok(())
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn un_emisor_que_se_para_se_corta_por_tiempo() {
        let (mut a, mut b) = tokio::io::duplex(1024);
        a.write_all(b"medio").await.unwrap();
        let mut fuera = Vec::new();
        let r = recibir_bytes(&mut b, &mut fuera, 100, |_| {}).await;
        match r {
            Err(ErrorTrama::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::TimedOut),
            otro => panic!("se esperaba un plazo: {otro:?}"),
        }
        // Lento pero sin pararse del todo, llega entero.
        let (mut a, mut b) = tokio::io::duplex(1024);
        let manda = tokio::spawn(async move {
            for _ in 0..4 {
                a.write_all(b"12345").await.unwrap();
                tokio::time::sleep(plazo_sin_bytes() / 3).await;
            }
        });
        let mut fuera = Vec::new();
        recibir_bytes(&mut b, &mut fuera, 20, |_| {}).await.unwrap();
        assert_eq!(fuera.len(), 20);
        manda.await.unwrap();
        // Y si corta antes, es un corte.
        let (a, mut b) = tokio::io::duplex(1024);
        drop(a);
        let r = recibir_bytes(&mut b, &mut Vec::new(), 10, |_| {}).await;
        assert!(
            matches!(r, Err(ErrorTrama::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof)
        );
    }

    fn deck(nombre: &str) -> DeckOfrecido {
        DeckOfrecido {
            nombre: nombre.into(),
            cartas: 100,
            proxies: true,
            tamano: 10,
            tamano_originales: 5,
            linaje: Some("abc-123".into()),
            actualizado: Some(1),
            portada: None,
            optimizado: false,
        }
    }

    #[tokio::test]
    async fn una_trama_va_y_vuelve_y_una_grande_no_se_lee() {
        let (mut a, mut b) = tokio::io::duplex(1 << 20);
        let m = AlReceptor::Hola {
            version: VERSION,
            secreto: Some("s".into()),
            pase: None,
            apodo: "Ana".into(),
        };
        escribir(&mut a, &m).await.unwrap();
        let leido: AlReceptor = leer(&mut b).await.unwrap();
        assert_eq!(leido, m);
        // Una trama que dice medir más del tope ni se lee.
        a.write_all(&((TOPE_TRAMA as u32) + 1).to_be_bytes())
            .await
            .unwrap();
        let e = leer::<_, AlReceptor>(&mut b).await.unwrap_err();
        assert!(matches!(e, ErrorTrama::Grande(_)));
        // Ni se escribe.
        let enorme = AlReceptor::Oferta {
            decks: (0..2_000).map(|i| deck(&format!("deck {i}"))).collect(),
        };
        assert!(escribir(&mut a, &enorme).await.is_err());
    }

    #[tokio::test]
    async fn un_json_roto_o_de_otro_tipo_es_un_error() {
        let (mut a, mut b) = tokio::io::duplex(4096);
        escribir_trama(&mut a, b"{no es json", TOPE_TRAMA)
            .await
            .unwrap();
        assert!(matches!(
            leer::<_, AlReceptor>(&mut b).await,
            Err(ErrorTrama::Json(_))
        ));
        escribir_trama(&mut a, br#"{"t":"respuesta","decisiones":[]}"#, TOPE_TRAMA)
            .await
            .unwrap();
        assert!(leer::<_, AlReceptor>(&mut b).await.is_err());
    }

    #[test]
    fn la_oferta_se_valida() {
        assert_eq!(validar_oferta(&[deck("Bien")]), Ok(()));
        assert_eq!(validar_oferta(&[]), Err("decks"));
        let muchos: Vec<_> = (0..=MAX_DECKS).map(|i| deck(&i.to_string())).collect();
        assert_eq!(validar_oferta(&muchos), Err("decks"));
        assert_eq!(validar_oferta(&[deck("  ")]), Err("nombre"));
        assert_eq!(validar_oferta(&[deck("a\nb")]), Err("nombre"));
        assert_eq!(
            validar_oferta(&[deck(&"x".repeat(MAX_NOMBRE + 1))]),
            Err("nombre")
        );
        // El peso no tiene tope: lo decide quien recibe al verlo.
        let mut d = deck("g");
        d.tamano = 40 * 1024 * 1024 * 1024;
        assert_eq!(validar_oferta(&[d]), Ok(()));
        let mut d = deck("l");
        d.linaje = Some("x'; --".into());
        assert_eq!(validar_oferta(&[d]), Err("linaje"));
        let mut d = deck("p");
        d.portada = Some("A".repeat(MAX_PORTADA + 1));
        assert_eq!(validar_oferta(&[d]), Err("portada"));
    }

    #[test]
    fn un_hola_sin_pase_sigue_valiendo_para_una_app_anterior() {
        // Una 3.5 no lo manda ni lo lee: el campo es opcional en los dos sentidos.
        let viejo = br#"{"t":"hola","version":1,"secreto":null,"apodo":"Ana"}"#;
        let h: AlReceptor = serde_json::from_slice(viejo).unwrap();
        assert!(matches!(h, AlReceptor::Hola { pase: None, .. }));
        let sin = serde_json::to_string(&AlReceptor::Hola {
            version: VERSION,
            secreto: None,
            pase: None,
            apodo: "Ana".into(),
        })
        .unwrap();
        assert!(!sin.contains("pase"));
    }

    #[test]
    fn el_apodo_de_fuera_se_limpia() {
        assert_eq!(apodo_limpio("  Ana \n\t  María  "), "Ana María");
        assert_eq!(apodo_limpio("a\u{0}b\u{1b}[31m"), "ab[31m");
        assert_eq!(apodo_limpio(&"z".repeat(100)).chars().count(), MAX_APODO);
    }

    #[tokio::test]
    async fn los_bytes_van_enteros_con_su_huella_y_un_corte_es_un_error() {
        let datos: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        let (mut tx, mut rx) = tokio::io::duplex(1 << 20);
        let mut fuente = std::io::Cursor::new(datos.clone());
        // `Cursor` no es AsyncRead de tokio; se usa un duplex intermedio.
        let (mut f_tx, mut f_rx) = tokio::io::duplex(1 << 20);
        f_tx.write_all(&datos).await.unwrap();
        drop(f_tx);
        let _ = &mut fuente;
        let mut avances = Vec::new();
        enviar_bytes(&mut f_rx, &mut tx, datos.len() as u64, |h| avances.push(h))
            .await
            .unwrap();
        let mut destino = Vec::new();
        let huella = recibir_bytes(&mut rx, &mut destino, datos.len() as u64, |_| {})
            .await
            .unwrap();
        assert_eq!(destino, datos);
        let esperada = hex(&Sha256::digest(&datos));
        assert_eq!(huella, esperada);
        assert_eq!(*avances.last().unwrap(), datos.len() as u64);
        // Si el otro corta antes de tiempo, error.
        let (mut tx, mut rx) = tokio::io::duplex(1 << 20);
        tx.write_all(&datos[..1000]).await.unwrap();
        drop(tx);
        let mut destino = Vec::new();
        assert!(recibir_bytes(&mut rx, &mut destino, 5_000, |_| {})
            .await
            .is_err());
    }
}
