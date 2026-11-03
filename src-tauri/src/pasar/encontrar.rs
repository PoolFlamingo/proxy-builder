//! **Encontrarse**: cómo dos equipos se reconocen sin escribir direcciones.
//!
//! - **Equipos cerca** (misma red local): quien abre «Recibir» se anuncia por
//!   mDNS con su apodo (`Anuncio`) y quien abre «Enviar» ve la lista
//!   (`Busqueda`). El servicio mDNS es propio (`pbdeck`), no el de iroh: solo
//!   se ven entre sí las Proxy Builder que están esperando un deck, no cada
//!   app hecha con iroh ni los nodos de una partida. Los dos ven el mismo
//!   código de 6 cifras (`codigo6`), como al emparejar por Bluetooth, y quien
//!   recibe lo acepta.
//! - **QR y código** (misma red o a distancia): el código `PBD1-…` (formato
//!   del código directo de la mesa, `red/codigo.rs`) lleva la dirección del
//!   nodo —con las locales y el relé— y un secreto de un solo uso.
//!
//! El anuncio y la búsqueda son independientes del `Endpoint` (no se
//! registran en él): el `Endpoint` es el de la mesa, uno por instalación, y
//! no se le añade nada que no se pueda quitar.

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::StreamExt;
use iroh::address_lookup::{AddressLookup, EndpointData};
use iroh::endpoint_info::UserData;
use iroh::{Endpoint, EndpointAddr, EndpointId, TransportAddr};
use iroh_mdns_address_lookup::{DiscoveryEvent, MdnsAddressLookup};
use sha2::{Digest, Sha256};
use tokio::task::AbortHandle;

use super::protocolo::{apodo_limpio, MAX_APODO};
use super::tipos::Cerca;
use crate::error::{AppError, AppResult, ErrorCode};

/// El nombre del servicio mDNS (`<id>._pbdeck._udp.local`).
const SERVICIO: &str = "pbdeck";
/// Cada cuánto se vuelve a publicar la dirección (por si cambia).
const REPUBLICAR: Duration = Duration::from_secs(3);
/// Cuántas direcciones IP lleva el código (más hacen un QR que no se lee).
pub const MAX_IP: usize = 6;

/// El código de 6 cifras que ven los dos equipos: 20 bits del SHA-256 de las
/// dos claves, ordenadas. No es un secreto (las claves son públicas): sirve
/// para que las personas comprueben que hablan con el equipo que creen.
pub fn codigo6(a: EndpointId, b: EndpointId) -> String {
    let (x, y) = if a.as_bytes() <= b.as_bytes() {
        (a, b)
    } else {
        (b, a)
    };
    let h = Sha256::new()
        .chain_update(x.as_bytes())
        .chain_update(y.as_bytes())
        .finalize();
    let bits = (u32::from(h[0]) << 12) | (u32::from(h[1]) << 4) | (u32::from(h[2]) >> 4);
    // 20 bits llegan a 1.048.575: siete cifras. Se queda en seis.
    format!("{:06}", bits % 1_000_000)
}

fn direccion_util(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => !(v.is_loopback() || v.is_unspecified() || v.is_link_local()),
        IpAddr::V6(v) => {
            let s = v.segments();
            !(v.is_loopback() || v.is_unspecified() || (s[0] & 0xffc0) == 0xfe80)
        }
    }
}

/// Lo bien que puede llegar a una dirección de la red local: primero las
/// privadas de IPv4, luego las demás de IPv4 y al final IPv6.
fn prioridad(ip: &IpAddr) -> u8 {
    match ip {
        IpAddr::V4(v) if v.is_private() => 0,
        IpAddr::V4(_) => 1,
        IpAddr::V6(_) => 2,
    }
}

/// La dirección para el código: con el relé (o los relés) y, de las directas,
/// solo las útiles y como mucho `max_ip` (las más probables primero).
pub fn recortar(addr: &EndpointAddr, max_ip: usize) -> EndpointAddr {
    let mut ips: Vec<TransportAddr> = addr
        .addrs
        .iter()
        .filter(|a| matches!(a, TransportAddr::Ip(s) if direccion_util(&s.ip())))
        .cloned()
        .collect();
    ips.sort_by_key(|a| match a {
        TransportAddr::Ip(s) => (prioridad(&s.ip()), s.ip(), s.port()),
        _ => (u8::MAX, IpAddr::from([0, 0, 0, 0]), 0),
    });
    ips.truncate(max_ip);
    let mut out = EndpointAddr::new(addr.id);
    out.addrs.extend(ips);
    out.addrs.extend(
        addr.addrs
            .iter()
            .filter(|a| matches!(a, TransportAddr::Relay(_)))
            .cloned(),
    );
    out
}

/// Solo el relé: para probar que el código vale a distancia.
#[cfg(test)]
pub fn sin_directas(addr: &EndpointAddr) -> EndpointAddr {
    let mut out = EndpointAddr::new(addr.id);
    out.addrs.extend(
        addr.addrs
            .iter()
            .filter(|a| matches!(a, TransportAddr::Relay(_)))
            .cloned(),
    );
    out
}

/// El QR del texto, como un SVG (negro sobre blanco, con su margen).
pub fn qr_svg(texto: &str) -> AppResult<String> {
    use qrcode::render::svg;
    use qrcode::{EcLevel, QrCode};
    let q = QrCode::with_error_correction_level(texto.as_bytes(), EcLevel::L)
        .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
    Ok(q.render::<svg::Color>()
        .min_dimensions(240, 240)
        .quiet_zone(true)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build())
}

fn mdns(id: EndpointId, anunciar: bool) -> AppResult<MdnsAddressLookup> {
    MdnsAddressLookup::builder()
        .service_name(SERVICIO)
        .advertise(anunciar)
        .build(id)
        .map_err(|e| crate::error::red_sin_http("mdns", Some(&e)))
}

/// Anunciarse en la red local con este apodo, mientras viva.
pub struct Anuncio {
    _mdns: MdnsAddressLookup,
    tarea: AbortHandle,
}

impl Anuncio {
    pub fn nuevo(ep: &Endpoint, apodo: &str) -> AppResult<Anuncio> {
        let m = mdns(ep.id(), true)?;
        let apodo: Option<UserData> = apodo_limpio(apodo).parse().ok();
        let ep = ep.clone();
        let publicador = m.clone();
        let tarea = tokio::spawn(async move {
            loop {
                let ips: Vec<TransportAddr> = ep
                    .addr()
                    .addrs
                    .into_iter()
                    .filter(|a| matches!(a, TransportAddr::Ip(s) if direccion_util(&s.ip())))
                    .collect();
                let mut datos = EndpointData::new(ips);
                datos.set_user_data(apodo.clone());
                publicador.publish(&datos);
                tokio::time::sleep(REPUBLICAR).await;
            }
        });
        Ok(Anuncio {
            _mdns: m,
            tarea: tarea.abort_handle(),
        })
    }
}

impl Drop for Anuncio {
    fn drop(&mut self) {
        self.tarea.abort();
    }
}

/// Los equipos que se anuncian cerca, mientras viva.
pub struct Busqueda {
    _mdns: MdnsAddressLookup,
    tarea: AbortHandle,
    encontrados: Arc<Mutex<BTreeMap<EndpointId, (String, EndpointAddr)>>>,
}

impl Busqueda {
    /// Empieza a buscar; `alcambiar` recibe la lista entera cada vez que cambia.
    pub async fn nueva(
        ep: &Endpoint,
        alcambiar: impl Fn(Vec<Cerca>) + Send + Sync + 'static,
    ) -> AppResult<Busqueda> {
        let m = mdns(ep.id(), false)?;
        let encontrados: Arc<Mutex<BTreeMap<EndpointId, (String, EndpointAddr)>>> = Arc::default();
        let mut eventos = m.subscribe().await;
        let lista = encontrados.clone();
        let yo = ep.id();
        let tarea = tokio::spawn(async move {
            while let Some(ev) = eventos.next().await {
                let cambio = {
                    let mut l = match lista.lock() {
                        Ok(l) => l,
                        Err(e) => e.into_inner(),
                    };
                    match ev {
                        DiscoveryEvent::Discovered { endpoint_info, .. } => {
                            let id = endpoint_info.endpoint_id;
                            if id == yo {
                                false
                            } else {
                                let apodo = endpoint_info
                                    .data
                                    .user_data()
                                    .map(|u| apodo_limpio(u.as_ref()))
                                    .unwrap_or_default();
                                l.insert(id, (apodo, endpoint_info.to_endpoint_addr()));
                                true
                            }
                        }
                        DiscoveryEvent::Expired { endpoint_id } => l.remove(&endpoint_id).is_some(),
                        _ => false,
                    }
                };
                if cambio {
                    alcambiar(lista_de(&lista));
                }
            }
        });
        Ok(Busqueda {
            _mdns: m,
            tarea: tarea.abort_handle(),
            encontrados,
        })
    }

    /// Una búsqueda que ya ha "visto" a un equipo (para los tests, sin mDNS).
    #[cfg(test)]
    pub fn de_prueba(id: EndpointId, apodo: &str, addr: EndpointAddr) -> Busqueda {
        let encontrados: Arc<Mutex<BTreeMap<EndpointId, (String, EndpointAddr)>>> = Arc::default();
        if let Ok(mut l) = encontrados.lock() {
            l.insert(id, (apodo.to_string(), addr));
        }
        Busqueda {
            _mdns: MdnsAddressLookup::builder()
                .service_name("pbdeck-prueba")
                .advertise(false)
                .build(iroh::SecretKey::generate().public())
                .expect("mDNS de prueba"),
            tarea: tokio::spawn(async {}).abort_handle(),
            encontrados,
        }
    }

    /// La dirección con la que llamar a uno de la lista.
    pub fn direccion(&self, id: &str) -> Option<EndpointAddr> {
        let l = match self.encontrados.lock() {
            Ok(l) => l,
            Err(e) => e.into_inner(),
        };
        l.iter()
            .find(|(k, _)| k.to_string() == id)
            .map(|(_, (_, a))| a.clone())
    }

    pub fn lista(&self) -> Vec<Cerca> {
        lista_de(&self.encontrados)
    }
}

impl Drop for Busqueda {
    fn drop(&mut self) {
        self.tarea.abort();
    }
}

fn lista_de(l: &Arc<Mutex<BTreeMap<EndpointId, (String, EndpointAddr)>>>) -> Vec<Cerca> {
    let l = match l.lock() {
        Ok(l) => l,
        Err(e) => e.into_inner(),
    };
    let mut v: Vec<Cerca> = l
        .iter()
        .map(|(id, (apodo, _))| Cerca {
            id: id.to_string(),
            apodo: apodo.clone(),
            huella: huella(id),
        })
        .collect();
    v.sort_by_key(|a| a.apodo.to_lowercase());
    v
}

/// Los primeros caracteres de una clave, para distinguir equipos.
pub fn huella(id: &EndpointId) -> String {
    id.fmt_short().to_string()
}

/// Lo más largo que se enseña de un apodo.
pub const APODO_MAX: usize = MAX_APODO;

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::SecretKey;

    #[test]
    fn el_codigo_de_6_cifras_es_el_mismo_en_los_dos_lados_y_siempre_de_6() {
        for _ in 0..200 {
            let a = SecretKey::generate().public();
            let b = SecretKey::generate().public();
            let c = codigo6(a, b);
            assert_eq!(c, codigo6(b, a));
            assert_eq!(c.len(), 6);
            assert!(c.chars().all(|x| x.is_ascii_digit()));
        }
        // Con otro equipo, otro código (casi siempre).
        let a = SecretKey::generate().public();
        let b = SecretKey::generate().public();
        let otros: std::collections::BTreeSet<String> = (0..30)
            .map(|_| codigo6(a, SecretKey::generate().public()))
            .collect();
        assert!(otros.len() > 20);
        let _ = b;
    }

    #[test]
    fn las_direcciones_del_codigo_se_recortan_y_conservan_el_rele() {
        let id = SecretKey::generate().public();
        let mut a = EndpointAddr::new(id);
        a.addrs.insert(TransportAddr::Relay(
            "https://relay.ejemplo.test".parse().unwrap(),
        ));
        for ip in [
            "127.0.0.1:1",
            "169.254.1.1:2",
            "[fe80::1]:3",
            "192.168.1.5:4",
            "10.0.0.2:5",
            "172.17.0.1:6",
            "83.4.5.6:7",
            "[2001:db8::1]:8",
            "192.168.1.6:9",
            "192.168.1.7:10",
            "192.168.1.8:11",
        ] {
            a.addrs.insert(TransportAddr::Ip(ip.parse().unwrap()));
        }
        let r = recortar(&a, MAX_IP);
        let ips: Vec<_> = r
            .addrs
            .iter()
            .filter_map(|x| match x {
                TransportAddr::Ip(s) => Some(*s),
                _ => None,
            })
            .collect();
        assert_eq!(ips.len(), MAX_IP);
        assert!(ips.iter().all(|s| s.ip().is_ipv4()));
        assert!(!ips.iter().any(|s| s.ip().is_loopback()));
        assert_eq!(r.addrs.iter().filter(|x| x.is_relay()).count(), 1);
        assert_eq!(r.id, id);
        assert!(sin_directas(&a).addrs.iter().all(TransportAddr::is_relay));
    }

    #[test]
    fn el_qr_sale_como_svg() {
        let svg = qr_svg("PBD1-abcdefghijklmnopqrstuvwxyz0123456789").unwrap();
        assert!(svg.contains("<svg") && svg.contains("</svg>"));
        // Un código de los de verdad cabe.
        let largo = format!("PBD1-{}", "a".repeat(900));
        assert!(qr_svg(&largo).is_ok());
    }

    #[tokio::test]
    async fn dos_equipos_se_ven_por_mdns_con_su_apodo() {
        // Necesita multicast en la máquina: en un contenedor sin red puede no
        // haberlo, y entonces el anuncio no llega (no es un fallo del código).
        let (a, b) = crate::red::nodo::tests::dos().await;
        let Ok(_anuncio) = Anuncio::nuevo(&a, "  Ana\n María ") else {
            eprintln!("sin mDNS en esta máquina: se salta");
            return;
        };
        let visto = Arc::new(Mutex::new(Vec::<Cerca>::new()));
        let v = visto.clone();
        let Ok(busqueda) = Busqueda::nueva(&b, move |l| {
            if let Ok(mut x) = v.lock() {
                *x = l;
            }
        })
        .await
        else {
            eprintln!("sin mDNS en esta máquina: se salta");
            return;
        };
        let mut encontrado = None;
        for _ in 0..60 {
            tokio::time::sleep(Duration::from_millis(250)).await;
            // Otras pruebas a la vez también se anuncian: se busca el de A.
            if let Some(c) = busqueda
                .lista()
                .into_iter()
                .find(|c| c.id == a.id().to_string())
            {
                encontrado = Some(c);
                break;
            }
        }
        let Some(c) = encontrado else {
            eprintln!("el mDNS no llegó en 15 s: se salta");
            return;
        };
        assert_eq!(c.id, a.id().to_string());
        assert_eq!(c.apodo, "Ana María");
        assert!(busqueda.direccion(&c.id).is_some_and(|d| d.id == a.id()));
    }
}
