//! **El protocolo de pasar decks** (`proxy-builder/deck/1`).
//!
//! Un stream bidireccional por intercambio. Tramas de `u32` (big endian) de
//! longitud + JSON, como el de la mesa (`red/protocolo.rs`), y después de un
//! `Paquete` van, sin trama, exactamente `tamano` bytes del `.pbdeck`.
//!
//! ```text
//! emisor                              receptor
//!   Hola { version, secreto?, apodo } →
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
//! (también la oferta), 32 decks por oferta, 500 MB por paquete.
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
/// Lo más grande que puede ser un paquete.
pub const MAX_PAQUETE: u64 = 500 * 1024 * 1024;
/// Lo más grande que puede ser todo lo que se manda de una vez.
pub const MAX_TOTAL: u64 = 2 * 1024 * 1024 * 1024;
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
        /// encuentro a mano (el código de 6 cifras).
        #[serde(default)]
        secreto: Option<String>,
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
    /// No se acepta (`motivo`: `version`, `codigo`, `ocupado`, `rechazado`,
    /// `caducado`).
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

/// Comprueba la oferta de fuera: cuántos decks, nombres, tamaños y linajes.
/// No toca la portada (la limpia quien la enseña, en Rust).
pub fn validar_oferta(decks: &[DeckOfrecido]) -> Result<(), &'static str> {
    if decks.is_empty() || decks.len() > MAX_DECKS {
        return Err("decks");
    }
    let mut total = 0u64;
    for d in decks {
        if !texto_sano(&d.nombre, MAX_NOMBRE) {
            return Err("nombre");
        }
        if d.cartas > 100_000 {
            return Err("cartas");
        }
        if d.tamano > MAX_PAQUETE || d.tamano_originales > MAX_PAQUETE {
            return Err("tamano");
        }
        total = total.saturating_add(d.tamano);
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
    if total > MAX_TOTAL {
        return Err("tamano");
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
/// lento no deja «Recibir» ocupado sin fin).
pub const PLAZO_SIN_BYTES: std::time::Duration = if cfg!(test) {
    std::time::Duration::from_millis(300)
} else {
    std::time::Duration::from_secs(30)
};

/// Copia exactamente `tamano` bytes de `r` a `w` por trozos, calculando su
/// SHA-256 y avisando del avance. Si el otro lado corta antes, es un error;
/// si pasa [`PLAZO_SIN_BYTES`] sin que llegue nada, un `Io` de `TimedOut`.
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
            let m = tokio::time::timeout(PLAZO_SIN_BYTES, r.read(&mut buf[leidos..n]))
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
                tokio::time::sleep(PLAZO_SIN_BYTES / 3).await;
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
        }
    }

    #[tokio::test]
    async fn una_trama_va_y_vuelve_y_una_grande_no_se_lee() {
        let (mut a, mut b) = tokio::io::duplex(1 << 20);
        let m = AlReceptor::Hola {
            version: VERSION,
            secreto: Some("s".into()),
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
        let mut d = deck("g");
        d.tamano = MAX_PAQUETE + 1;
        assert_eq!(validar_oferta(&[d]), Err("tamano"));
        let mut d = deck("l");
        d.linaje = Some("x'; --".into());
        assert_eq!(validar_oferta(&[d]), Err("linaje"));
        let mut d = deck("p");
        d.portada = Some("A".repeat(MAX_PORTADA + 1));
        assert_eq!(validar_oferta(&[d]), Err("portada"));
        // La suma de lo que se manda también tiene tope.
        let grandes: Vec<_> = (0..5)
            .map(|i| {
                let mut d = deck(&format!("g{i}"));
                d.tamano = MAX_PAQUETE;
                d
            })
            .collect();
        assert_eq!(validar_oferta(&grandes), Err("tamano"));
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
