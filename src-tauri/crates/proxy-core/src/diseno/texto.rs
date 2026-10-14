//! **Texto enriquecido y maquetación.**
//!
//! - `reglas` y `sabor` pasan el texto de Scryfall a párrafos de trozos:
//!   texto normal, cursiva (recordatorios entre paréntesis, palabras de
//!   habilidad, sabor) y símbolos `{X}`.
//! - `Maquetador` los coloca con `parley`. Los símbolos van como cajas en
//!   línea; los que van seguidos («{2}{U}{U}:») forman **una sola caja** con
//!   la puntuación pegada, para que nunca se partan entre dos líneas.
//! - `caja_de_reglas` apila reglas y sabor (con la raya entre medias) y
//!   `ajustar_reglas` baja la letra hasta que quepa.
//! - `linea_ajustada` es para el título y la línea de tipo: primero se
//!   estrecha a lo ancho y solo después baja el tamaño (§3.7 del plan).

use parley::{
    Alignment, AlignmentOptions, FontFamily, FontStyle, FontWeight, InlineBox, InlineBoxKind,
    Layout, LayoutContext, LineHeight, StyleProperty, VerticalAlign,
};

use super::letras::Letras;
use super::Color;

/// Un trozo de un párrafo.
#[derive(Debug, Clone, PartialEq)]
pub enum Trozo {
    Texto { texto: String, cursiva: bool },
    Simbolo(String),
}

pub type Parrafo = Vec<Trozo>;

fn empujar(p: &mut Parrafo, texto: &str, cursiva: bool) {
    if texto.is_empty() {
        return;
    }
    if let Some(Trozo::Texto {
        texto: t,
        cursiva: c,
    }) = p.last_mut()
    {
        if *c == cursiva {
            t.push_str(texto);
            return;
        }
    }
    p.push(Trozo::Texto {
        texto: texto.to_string(),
        cursiva,
    });
}

/// Una línea con símbolos y paréntesis en cursiva. `base` es la cursiva de
/// fuera de los paréntesis (el sabor va entero en cursiva).
fn linea(texto: &str, base: bool) -> Parrafo {
    let mut p = Parrafo::new();
    let mut nivel = 0usize;
    let mut resto = texto;
    while let Some(i) = resto.find(['{', '(', ')']) {
        let (antes, desde) = resto.split_at(i);
        empujar(&mut p, antes, base || nivel > 0);
        let c = desde.as_bytes()[0];
        match c {
            b'{' => match desde.find('}') {
                Some(f) => {
                    p.push(Trozo::Simbolo(super::simbolos::codigo(&desde[..=f])));
                    resto = &desde[f + 1..];
                    continue;
                }
                None => {
                    empujar(&mut p, desde, base || nivel > 0);
                    return p;
                }
            },
            b'(' => {
                nivel += 1;
                empujar(&mut p, "(", true);
            }
            _ => {
                empujar(&mut p, ")", base || nivel > 0);
                nivel = nivel.saturating_sub(1);
            }
        }
        resto = &desde[1..];
    }
    empujar(&mut p, resto, base || nivel > 0);
    p
}

/// La palabra de habilidad del principio de una línea («Imprimir — …»,
/// «Landfall — …»): corta, sin símbolos ni dos puntos.
fn palabra_de_habilidad(texto: &str) -> Option<usize> {
    let i = texto.find(" — ")?;
    let antes = &texto[..i];
    let palabras = antes.split_whitespace().count();
    (palabras > 0 && palabras <= 4 && !antes.contains(['{', ':', '(', '.', ',', '•'])).then_some(i)
}

/// El texto de reglas, un párrafo por línea.
pub fn reglas(texto: &str) -> Vec<Parrafo> {
    texto
        .split('\n')
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| match palabra_de_habilidad(l) {
            Some(i) => {
                let mut p = Parrafo::new();
                empujar(&mut p, &l[..i], true);
                for t in linea(&l[i..], false) {
                    match t {
                        Trozo::Texto { texto, cursiva } => empujar(&mut p, &texto, cursiva),
                        s => p.push(s),
                    }
                }
                p
            }
            None => linea(l, false),
        })
        .collect()
}

/// El texto de sabor: todo en cursiva, un párrafo por línea (la atribución
/// «—Autor» de Scryfall ya viene en su línea).
pub fn sabor(texto: &str) -> Vec<Parrafo> {
    texto
        .split('\n')
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| linea(l, true))
        .collect()
}

/// Una línea con símbolos `{X}` y nada más (sin cursivas): el pie, la
/// línea de tipo.
pub fn con_simbolos(texto: &str) -> Parrafo {
    let mut p = Parrafo::new();
    let mut resto = texto;
    while let Some(i) = resto.find('{') {
        let Some(f) = resto[i..].find('}') else {
            break;
        };
        empujar(&mut p, &resto[..i], false);
        p.push(Trozo::Simbolo(super::simbolos::codigo(&resto[i..=i + f])));
        resto = &resto[i + f + 1..];
    }
    empujar(&mut p, resto, false);
    p
}

/// Un párrafo de texto llano, sin cursivas ni símbolos.
pub fn llano(texto: &str) -> Parrafo {
    vec![Trozo::Texto {
        texto: texto.to_string(),
        cursiva: false,
    }]
}

/// Alineación de un bloque.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alinear {
    #[default]
    Inicio,
    Centro,
    Fin,
}

/// Cómo se escribe un bloque. Tamaños en píxeles de la salida.
#[derive(Debug, Clone, PartialEq)]
pub struct Estilo {
    pub familia: String,
    /// 400 normal, 700 negrita (sirve con letras variables).
    pub peso: f32,
    pub tam: f32,
    /// Interlínea, en múltiplos del tamaño.
    pub interlinea: f32,
    pub color: Color,
    pub alinear: Alinear,
    /// Versalitas de verdad si la letra las tiene (`smcp`).
    pub versalitas: bool,
}

impl Estilo {
    pub fn con_tam(&self, tam: f32) -> Estilo {
        Estilo {
            tam,
            ..self.clone()
        }
    }
}

/// Lado de los símbolos en el texto, en múltiplos del tamaño de la letra.
pub const LADO_SIMBOLO: f32 = 0.80;
/// Separación entre dos símbolos seguidos.
pub const HUECO_SIMBOLO: f32 = 0.05;
/// Cuánto baja el símbolo por debajo de la línea base.
pub const BAJA_SIMBOLO: f32 = 0.09;

/// Símbolos seguidos que van juntos, con la puntuación que les sigue.
pub(crate) struct Grupo {
    pub(crate) simbolos: Vec<String>,
    pub(crate) lado: f32,
    pub(crate) hueco: f32,
    pub(crate) cola: Option<Box<Bloque>>,
}

/// Un párrafo ya maquetado.
pub struct Bloque {
    pub(crate) layout: Layout<Color>,
    pub(crate) grupos: Vec<Grupo>,
    pub ancho: f32,
    pub alto: f32,
    /// Distancia del borde de arriba a la primera línea base.
    pub base: f32,
}

impl Bloque {
    /// Hasta dónde llega el último renglón (px desde el borde izquierdo).
    pub fn ultima_linea_ancho(&self) -> f32 {
        self.layout
            .lines()
            .last()
            .map_or(0.0, |l| l.metrics().offset + l.metrics().advance)
    }
}

/// Contexto de maquetación (se reutiliza entre bloques).
#[derive(Default)]
pub struct Maquetador {
    lcx: LayoutContext<Color>,
}

impl Maquetador {
    pub fn nuevo() -> Maquetador {
        Maquetador::default()
    }

    /// Maqueta un párrafo con ancho máximo (`None` = una sola línea).
    pub fn maquetar(
        &mut self,
        letras: &mut Letras,
        parrafo: &Parrafo,
        estilo: &Estilo,
        ancho: Option<f32>,
    ) -> Bloque {
        // Texto plano y dónde van los grupos de símbolos.
        let mut texto = String::new();
        let mut cursivas: Vec<std::ops::Range<usize>> = Vec::new();
        let mut grupos: Vec<(usize, Vec<String>, String)> = Vec::new();
        let mut i = 0;
        while i < parrafo.len() {
            match &parrafo[i] {
                Trozo::Texto { texto: t, cursiva } => {
                    let ini = texto.len();
                    texto.push_str(t);
                    if *cursiva {
                        cursivas.push(ini..texto.len());
                    }
                    i += 1;
                }
                Trozo::Simbolo(_) => {
                    let mut simbolos = Vec::new();
                    while let Some(Trozo::Simbolo(s)) = parrafo.get(i) {
                        simbolos.push(s.clone());
                        i += 1;
                    }
                    // La puntuación pegada («:», «,», «.») va dentro del grupo.
                    let mut cola = String::new();
                    if let Some(Trozo::Texto { texto: t, cursiva }) = parrafo.get(i) {
                        let n: usize = t
                            .chars()
                            .take_while(|c| matches!(c, ':' | ',' | '.' | ';' | ')'))
                            .map(char::len_utf8)
                            .sum();
                        if n > 0 {
                            cola = t[..n].to_string();
                            let resto = t[n..].to_string();
                            let cursiva = *cursiva;
                            let ini = texto.len();
                            grupos.push((ini, simbolos, cola));
                            texto.push_str(&resto);
                            if cursiva {
                                cursivas.push(ini..texto.len());
                            }
                            i += 1;
                            continue;
                        }
                    }
                    grupos.push((texto.len(), simbolos, cola));
                }
            }
        }

        let lado = estilo.tam * LADO_SIMBOLO;
        let hueco = estilo.tam * HUECO_SIMBOLO;
        let mut hechos = Vec::new();
        let mut cajas = Vec::new();
        for (id, (indice, simbolos, cola)) in grupos.into_iter().enumerate() {
            let n = simbolos.len() as f32;
            let cola = (!cola.is_empty())
                .then(|| Box::new(self.maquetar(letras, &llano(&cola), estilo, None)));
            let ancho_cola = cola.as_ref().map_or(0.0, |c| c.ancho + hueco);
            let ancho_caja = n * lado + (n - 1.0) * hueco + ancho_cola;
            cajas.push(InlineBox {
                id: id as u64,
                kind: InlineBoxKind::InFlow,
                index: indice,
                width: ancho_caja,
                height: lado,
                baseline: Some(lado - estilo.tam * BAJA_SIMBOLO),
                vertical_align: VerticalAlign::BASELINE,
            });
            hechos.push(Grupo {
                simbolos,
                lado,
                hueco,
                cola,
            });
        }

        let mut b = self.lcx.ranged_builder(&mut letras.fcx, &texto, 1.0, false);
        b.push_default(StyleProperty::FontFamily(FontFamily::named(
            &estilo.familia,
        )));
        b.push_default(StyleProperty::FontSize(estilo.tam));
        b.push_default(StyleProperty::FontWeight(FontWeight::new(estilo.peso)));
        b.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
            estilo.interlinea,
        )));
        b.push_default(StyleProperty::Brush(estilo.color));
        // Cifras de caja alta siempre («3/5», «{2}»): las elzevirianas bajan
        // de la línea y en una carta quedan mal.
        b.push_default(StyleProperty::FontFeatures(parley::FontFeatures::from(
            if estilo.versalitas {
                "\"lnum\" on, \"smcp\" on"
            } else {
                "\"lnum\" on"
            },
        )));
        for r in cursivas {
            if !r.is_empty() {
                b.push(StyleProperty::FontStyle(FontStyle::Italic), r);
            }
        }
        for c in cajas {
            b.push_inline_box(c);
        }
        let mut layout = b.build(&texto);
        layout.break_all_lines(ancho);
        let alinear = match estilo.alinear {
            Alinear::Inicio => Alignment::Start,
            Alinear::Centro => Alignment::Center,
            Alinear::Fin => Alignment::End,
        };
        layout.align(alinear, AlignmentOptions::default());
        let base = layout
            .lines()
            .next()
            .map_or(estilo.tam, |l| l.metrics().baseline);
        Bloque {
            ancho: layout.width(),
            alto: layout.height(),
            base,
            layout,
            grupos: hechos,
        }
    }
}

/// Una línea (título, tipo) ajustada a un ancho: primero se estrecha hasta
/// `estrechado_min` y, si no basta, baja el tamaño. Devuelve el bloque, el
/// factor de estrechado y si hubo que bajar el tamaño.
pub fn linea_ajustada(
    m: &mut Maquetador,
    letras: &mut Letras,
    parrafo: &Parrafo,
    estilo: &Estilo,
    ancho: f32,
    estrechado_min: f32,
) -> (Bloque, f32, bool) {
    let b = m.maquetar(letras, parrafo, estilo, None);
    if b.ancho <= ancho || b.ancho <= 0.0 {
        return (b, 1.0, false);
    }
    let factor = ancho / b.ancho;
    if factor >= estrechado_min {
        return (b, factor, false);
    }
    // Ni estrechando al máximo cabe: se baja el tamaño lo justo.
    let tam = estilo.tam * factor / estrechado_min;
    let b = m.maquetar(letras, parrafo, &estilo.con_tam(tam), None);
    let factor = (ancho / b.ancho).min(1.0);
    (b, factor, true)
}

/// Reglas y sabor maquetados y apilados.
pub struct CajaTexto {
    /// Cada bloque con su posición vertical (borde de arriba).
    pub bloques: Vec<(Bloque, f32)>,
    /// Dónde va la raya entre reglas y sabor, si hay sabor y reglas.
    pub raya: Option<f32>,
    pub alto: f32,
    /// Tamaño final de la letra (px).
    pub tam: f32,
}

/// Separaciones de la caja de reglas, en múltiplos del tamaño.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Separaciones {
    pub parrafo: f32,
    /// Espacio total de la raya del sabor (repartido arriba y abajo).
    pub sabor: f32,
}

impl Default for Separaciones {
    fn default() -> Self {
        Separaciones {
            parrafo: 0.32,
            sabor: 0.75,
        }
    }
}

/// Maqueta reglas y sabor a un tamaño dado.
pub fn caja_de_reglas(
    m: &mut Maquetador,
    letras: &mut Letras,
    reglas: &[Parrafo],
    sabor: &[Parrafo],
    estilo: &Estilo,
    ancho: f32,
    sep: Separaciones,
) -> CajaTexto {
    let mut bloques = Vec::new();
    let mut y = 0.0;
    for (i, p) in reglas.iter().enumerate() {
        if i > 0 {
            y += sep.parrafo * estilo.tam;
        }
        let b = m.maquetar(letras, p, estilo, Some(ancho));
        let alto = b.alto;
        bloques.push((b, y));
        y += alto;
    }
    let mut raya = None;
    if !sabor.is_empty() {
        if !reglas.is_empty() {
            raya = Some(y + sep.sabor * estilo.tam / 2.0);
            y += sep.sabor * estilo.tam;
        }
        for (i, p) in sabor.iter().enumerate() {
            if i > 0 {
                y += sep.parrafo * 0.4 * estilo.tam;
            }
            let b = m.maquetar(letras, p, estilo, Some(ancho));
            let alto = b.alto;
            bloques.push((b, y));
            y += alto;
        }
    }
    CajaTexto {
        bloques,
        raya,
        alto: y,
        tam: estilo.tam,
    }
}

/// El resultado de ajustar las reglas.
pub struct Ajuste {
    pub caja: CajaTexto,
    /// Lo que sobra por abajo (px) si ni al mínimo cabe; 0 si cabe.
    pub sobra: f32,
}

/// Baja la letra de 0,1 pt en 0,1 pt (`paso`, en px) desde `estilo.tam`
/// hasta `tam_min` hasta que la caja quepa en `alto_max`. Si al mínimo no
/// cabe, aprieta los párrafos y, si aun así no, devuelve lo que sobra.
#[allow(clippy::too_many_arguments)]
pub fn ajustar_reglas(
    m: &mut Maquetador,
    letras: &mut Letras,
    reglas: &[Parrafo],
    sabor: &[Parrafo],
    estilo: &Estilo,
    ancho: f32,
    alto_max: f32,
    tam_min: f32,
    paso: f32,
) -> Ajuste {
    let sep = Separaciones::default();
    let caja = caja_de_reglas(m, letras, reglas, sabor, estilo, ancho, sep);
    if caja.alto <= alto_max {
        return Ajuste { caja, sobra: 0.0 };
    }
    // Búsqueda binaria sobre los pasos (el alto baja con el tamaño).
    let pasos = ((estilo.tam - tam_min) / paso).floor().max(0.0) as u32;
    let mut lo = 1u32; // primer paso que se prueba
    let mut hi = pasos;
    let mut mejor: Option<CajaTexto> = None;
    while lo <= hi {
        let mid = (lo + hi) / 2;
        let tam = estilo.tam - mid as f32 * paso;
        let c = caja_de_reglas(m, letras, reglas, sabor, &estilo.con_tam(tam), ancho, sep);
        if c.alto <= alto_max {
            mejor = Some(c);
            if mid == 0 {
                break;
            }
            hi = mid - 1;
        } else {
            lo = mid + 1;
        }
    }
    if let Some(caja) = mejor {
        return Ajuste { caja, sobra: 0.0 };
    }
    // Al mínimo: se aprietan los párrafos.
    let apretado = Separaciones {
        parrafo: 0.12,
        sabor: 0.45,
    };
    let caja = caja_de_reglas(
        m,
        letras,
        reglas,
        sabor,
        &estilo.con_tam(tam_min),
        ancho,
        apretado,
    );
    let sobra = (caja.alto - alto_max).max(0.0);
    Ajuste { caja, sobra }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str, c: bool) -> Trozo {
        Trozo::Texto {
            texto: s.into(),
            cursiva: c,
        }
    }

    #[test]
    fn simbolos_y_recordatorio() {
        let p = reglas("{T}: Agrega {G}. (Puedes hacerlo.)");
        assert_eq!(
            p,
            vec![vec![
                Trozo::Simbolo("T".into()),
                t(": Agrega ", false),
                Trozo::Simbolo("G".into()),
                t(". ", false),
                t("(Puedes hacerlo.)", true),
            ]]
        );
    }

    #[test]
    fn palabra_de_habilidad_en_cursiva() {
        let p = reglas("Imprimir — Cuando entre, roba una carta.\nElige uno —\n• Roba.");
        assert_eq!(p[0][0], t("Imprimir", true));
        assert_eq!(p[0][1], t(" — Cuando entre, roba una carta.", false));
        // «Elige uno —» no es palabra de habilidad.
        assert_eq!(p[1], vec![t("Elige uno —", false)]);
        assert_eq!(p.len(), 3);
    }

    #[test]
    fn sabor_entero_en_cursiva() {
        let s = sabor("«Te daré mi destrucción.»\n—Veldanava");
        assert_eq!(s.len(), 2);
        assert!(s
            .iter()
            .flatten()
            .all(|x| matches!(x, Trozo::Texto { cursiva: true, .. })));
    }

    #[test]
    fn pie_con_iconos_sin_cursivas() {
        assert_eq!(
            con_simbolos("RTS{ESTRELLA}ES (Isekai)"),
            vec![
                t("RTS", false),
                Trozo::Simbolo("ESTRELLA".into()),
                t("ES (Isekai)", false)
            ]
        );
    }

    #[test]
    fn llaves_sin_cerrar_no_rompen() {
        assert_eq!(reglas("Coste {2"), vec![vec![t("Coste {2", false)]]);
    }

    // ── Maquetación ──

    use crate::diseno::render::Motor;

    fn estilo(tam: f32) -> Estilo {
        Estilo {
            familia: "Spectral".into(),
            peso: 400.0,
            tam,
            interlinea: 1.06,
            color: [0, 0, 0, 255],
            alinear: Alinear::Inicio,
            versalitas: false,
        }
    }

    #[test]
    fn una_linea_que_cabe_no_se_toca() {
        let mut m = Motor::de_fabrica().unwrap();
        let (b, f, reducido) = linea_ajustada(
            &mut m.m,
            &mut m.letras,
            &llano("Ocaso"),
            &estilo(30.0),
            1000.0,
            0.6,
        );
        assert_eq!((f, reducido), (1.0, false));
        assert!(b.ancho > 0.0 && b.ancho < 1000.0);
    }

    #[test]
    fn una_linea_algo_larga_se_estrecha_sin_bajar_la_letra() {
        let mut m = Motor::de_fabrica().unwrap();
        let p = llano("Crepúsculo de Veldora");
        let natural = m.m.maquetar(&mut m.letras, &p, &estilo(30.0), None).ancho;
        let (b, f, reducido) = linea_ajustada(
            &mut m.m,
            &mut m.letras,
            &p,
            &estilo(30.0),
            natural * 0.8,
            0.6,
        );
        assert!(!reducido);
        assert!((f - 0.8).abs() < 1e-3, "{f}");
        assert!((b.ancho * f - natural * 0.8).abs() < 0.5);
    }

    #[test]
    fn una_linea_muy_larga_baja_la_letra_y_cabe() {
        let mut m = Motor::de_fabrica().unwrap();
        let p = llano("Crepúsculo de Veldora, el dragón de la tormenta");
        let e = estilo(30.0);
        let natural = m.m.maquetar(&mut m.letras, &p, &e, None).ancho;
        let ancho = natural * 0.3;
        let (b, f, reducido) = linea_ajustada(&mut m.m, &mut m.letras, &p, &e, ancho, 0.6);
        assert!(reducido);
        assert!(f <= 1.0);
        assert!(b.ancho * f <= ancho + 0.5, "{} > {ancho}", b.ancho * f);
        assert!(b.alto < m.m.maquetar(&mut m.letras, &p, &e, None).alto);
    }

    #[test]
    fn un_titulo_vacio_cabe_sin_estrechar() {
        // Una carta sin nombre temático todavía: no se estrecha ni se avisa.
        let mut m = Motor::de_fabrica().unwrap();
        let (b, f, reducido) = linea_ajustada(
            &mut m.m,
            &mut m.letras,
            &llano(""),
            &estilo(30.0),
            300.0,
            0.6,
        );
        assert_eq!((f, reducido), (1.0, false));
        assert!(b.ancho.is_finite() && b.ancho < 300.0);
    }

    #[test]
    fn la_raya_del_sabor_solo_va_entre_reglas_y_sabor() {
        let mut m = Motor::de_fabrica().unwrap();
        let e = estilo(24.0);
        let sep = Separaciones::default();
        let r = reglas("Vuela.");
        let s = sabor("«Hola.»");
        let ambas = caja_de_reglas(&mut m.m, &mut m.letras, &r, &s, &e, 400.0, sep);
        let raya = ambas.raya.unwrap();
        // La raya cae entre el último bloque de reglas y el primero del sabor.
        let (b0, y0) = &ambas.bloques[0];
        assert!(raya > y0 + b0.alto && raya < ambas.bloques[1].1);
        let solo_sabor = caja_de_reglas(&mut m.m, &mut m.letras, &[], &s, &e, 400.0, sep);
        assert!(solo_sabor.raya.is_none());
        assert_eq!(solo_sabor.bloques[0].1, 0.0, "sin hueco arriba");
        let solo_reglas = caja_de_reglas(&mut m.m, &mut m.letras, &r, &[], &e, 400.0, sep);
        assert!(solo_reglas.raya.is_none());
        let nada = caja_de_reglas(&mut m.m, &mut m.letras, &[], &[], &e, 400.0, sep);
        assert_eq!((nada.alto, nada.bloques.len()), (0.0, 0));
    }

    #[test]
    fn los_parrafos_se_apilan_con_su_separacion() {
        let mut m = Motor::de_fabrica().unwrap();
        let e = estilo(20.0);
        let r = reglas("Vuela.\nPrisa.\nVínculo vital.");
        let c = caja_de_reglas(
            &mut m.m,
            &mut m.letras,
            &r,
            &[],
            &e,
            400.0,
            Separaciones::default(),
        );
        assert_eq!(c.bloques.len(), 3);
        for w in c.bloques.windows(2) {
            let ((a, ya), (_, yb)) = (&w[0], &w[1]);
            assert!((yb - (ya + a.alto + 0.32 * 20.0)).abs() < 1e-3);
        }
        let (u, yu) = c.bloques.last().unwrap();
        assert!((c.alto - (yu + u.alto)).abs() < 1e-3);
        assert_eq!(c.tam, 20.0);
    }

    #[test]
    fn el_ultimo_renglon_de_un_parrafo_partido_es_mas_corto_que_la_caja() {
        let mut m = Motor::de_fabrica().unwrap();
        let p = &reglas(&"Roba una carta. ".repeat(10))[0];
        let b = m.m.maquetar(&mut m.letras, p, &estilo(20.0), Some(300.0));
        assert!(b.alto > 40.0, "ocupa varias líneas");
        let fin = b.ultima_linea_ancho();
        assert!(fin > 0.0 && fin <= 300.0 + 0.5, "{fin}");
    }

    #[test]
    fn ajustar_las_reglas_baja_la_letra_hasta_que_caben() {
        let mut m = Motor::de_fabrica().unwrap();
        let e = estilo(26.0);
        let r = reglas(&"Siempre que una criatura entre, roba una carta. ".repeat(5));
        let s = sabor("«Saber es poder.»");
        let natural = caja_de_reglas(
            &mut m.m,
            &mut m.letras,
            &r,
            &s,
            &e,
            400.0,
            Separaciones::default(),
        )
        .alto;
        // Cabe: igual.
        let a = ajustar_reglas(
            &mut m.m,
            &mut m.letras,
            &r,
            &s,
            &e,
            400.0,
            natural + 1.0,
            10.0,
            0.5,
        );
        assert_eq!((a.caja.tam, a.sobra), (26.0, 0.0));
        // Apretado: baja, cabe y medio punto más ya no cabría.
        let max = natural * 0.75;
        let a = ajustar_reglas(&mut m.m, &mut m.letras, &r, &s, &e, 400.0, max, 10.0, 0.5);
        assert_eq!(a.sobra, 0.0);
        assert!(a.caja.alto <= max && a.caja.tam < 26.0 && a.caja.tam >= 10.0);
        let mayor = caja_de_reglas(
            &mut m.m,
            &mut m.letras,
            &r,
            &s,
            &e.con_tam(a.caja.tam + 0.5),
            400.0,
            Separaciones::default(),
        );
        assert!(mayor.alto > max);
        // Imposible: al mínimo, apretado y diciendo cuánto sobra.
        let a = ajustar_reglas(&mut m.m, &mut m.letras, &r, &s, &e, 400.0, 10.0, 14.0, 0.5);
        assert_eq!(a.caja.tam, 14.0);
        assert!((a.sobra - (a.caja.alto - 10.0)).abs() < 1e-3 && a.sobra > 0.0);
        // Un mínimo por encima del tamaño no hace crecer el texto más de lo pedido.
        let a = ajustar_reglas(
            &mut m.m,
            &mut m.letras,
            &r,
            &s,
            &e,
            400.0,
            natural + 1.0,
            40.0,
            0.5,
        );
        assert_eq!(a.caja.tam, 26.0);
    }
}
