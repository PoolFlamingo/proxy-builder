//! **Auto ordenar** (el «Auto Arrange» de fabricard).
//!
//! - Quita los huecos en blanco que no estén fijados.
//! - Junta las copias de la misma carta, en el orden en que aparece cada una
//!   por primera vez.
//! - Deja las cartas de doble cara juntas, después de las de una cara.
//! - 🛑 Las cartas **fijadas no se mueven** de su posición (si el deck queda
//!   más corto que su posición, van al final, en su orden).

use std::collections::HashMap;

use crate::model::{CardSource, DeckCard};

/// Lo que hace que dos copias sean «la misma carta».
fn clave(c: &DeckCard) -> String {
    match &c.source {
        CardSource::Scryfall { .. } | CardSource::Token { .. } => {
            format!("s:{}", c.meta.name.to_lowercase())
        }
        // Las propias se agrupan por la imagen: el nombre es el del fichero.
        CardSource::Custom => match &c.front {
            Some(img) => format!("u:{}", img.blob),
            None => format!("u:{}", c.id),
        },
        CardSource::Blank => format!("b:{}", c.id),
    }
}

/// El nuevo orden, como lista de ids. Los huecos no fijados desaparecen.
pub fn arrange(cartas: &[DeckCard]) -> Vec<String> {
    let quedan: Vec<(usize, &DeckCard)> = cartas
        .iter()
        .enumerate()
        .filter(|(_, c)| c.locked || !matches!(c.source, CardSource::Blank))
        .collect();
    let n = quedan.len();

    // Grupos en orden de primera aparición; las DFC, después.
    let mut orden_grupos: Vec<String> = Vec::new();
    let mut grupos: HashMap<String, Vec<&DeckCard>> = HashMap::new();
    for (_, c) in quedan.iter().filter(|(_, c)| !c.locked) {
        let k = clave(c);
        if !grupos.contains_key(&k) {
            orden_grupos.push(k.clone());
        }
        grupos.entry(k).or_default().push(c);
    }
    let es_dfc = |k: &String| {
        grupos
            .get(k)
            .and_then(|g| g.first())
            .is_some_and(|c| c.meta.is_dfc)
    };
    let (dfc, simples): (Vec<String>, Vec<String>) = orden_grupos.into_iter().partition(es_dfc);
    let mut libres = simples
        .iter()
        .chain(dfc.iter())
        .flat_map(|k| grupos.get(k).into_iter().flatten())
        .map(|c| c.id.clone());

    // Las fijadas, en su sitio si cabe; si no, al final.
    let mut huecos: Vec<Option<String>> = vec![None; n];
    let mut sin_sitio = Vec::new();
    for (i, c) in quedan.iter().filter(|(_, c)| c.locked) {
        if *i < n && huecos[*i].is_none() {
            huecos[*i] = Some(c.id.clone());
        } else {
            sin_sitio.push(c.id.clone());
        }
    }
    let mut sin_sitio = sin_sitio.into_iter();
    huecos
        .into_iter()
        .filter_map(|h| h.or_else(|| libres.next()).or_else(|| sin_sitio.next()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn carta(id: &str, nombre: &str) -> DeckCard {
        DeckCard {
            id: id.into(),
            source: CardSource::Scryfall {
                scryfall_id: format!("sf-{nombre}"),
            },
            front: None,
            back: BackSpec::DeckDefault,
            edits: CardEdits::default(),
            locked: false,
            meta: CardMeta {
                name: nombre.into(),
                ..CardMeta::default()
            },
        }
    }

    fn hueco(id: &str) -> DeckCard {
        DeckCard {
            source: CardSource::Blank,
            ..carta(id, "")
        }
    }

    #[test]
    fn junta_copias_y_quita_huecos() {
        let cs = vec![
            carta("1", "Sol Ring"),
            carta("2", "Forest"),
            hueco("h"),
            carta("3", "sol ring"),
            carta("4", "Forest"),
        ];
        assert_eq!(arrange(&cs), ["1", "3", "2", "4"]);
    }

    #[test]
    fn las_dfc_van_juntas_al_final() {
        let mut d = carta("d1", "Delver");
        d.meta.is_dfc = true;
        let mut d2 = d.clone();
        d2.id = "d2".into();
        let cs = vec![d, carta("a", "A"), d2, carta("b", "B")];
        assert_eq!(arrange(&cs), ["a", "b", "d1", "d2"]);
    }

    #[test]
    fn las_fijadas_no_se_mueven() {
        let mut cs: Vec<DeckCard> = (0..8)
            .map(|i| carta(&format!("c{i}"), if i % 2 == 0 { "X" } else { "Y" }))
            .collect();
        cs[0].locked = true;
        cs[5].locked = true;
        let r = arrange(&cs);
        assert_eq!(r.len(), 8);
        assert_eq!(r[0], "c0");
        assert_eq!(r[5], "c5");
        // El resto, agrupado en orden de primera aparición: Y (c1 es la
        // primera libre) y luego X.
        let resto: Vec<&str> = r
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 0 && *i != 5)
            .map(|(_, s)| s.as_str())
            .collect();
        assert_eq!(resto, ["c1", "c3", "c7", "c2", "c4", "c6"]);
    }

    #[test]
    fn una_fijada_que_ya_no_cabe_va_al_final() {
        let mut cs = vec![hueco("h1"), hueco("h2"), carta("a", "A"), carta("b", "B")];
        cs[3].locked = true;
        // Quedan 2: «a» y la fijada «b», que estaba en la 3.
        assert_eq!(arrange(&cs), ["a", "b"]);
    }

    #[test]
    fn un_hueco_fijado_se_queda() {
        let mut cs = vec![carta("a", "A"), hueco("h"), carta("b", "A")];
        cs[1].locked = true;
        assert_eq!(arrange(&cs), ["a", "h", "b"]);
    }
}
