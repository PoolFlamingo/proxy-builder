//! **Las palabras fijas que pinta el motor**, por idioma de diseño: la de los
//! niveles de un leveler y el recordatorio de Fusionar.
//!
//! En los idiomas con cartas impresas salen de cartas reales en Scryfall
//! (anotadas al lado). En rumano, polaco, neerlandés y ucraniano no hay
//! cartas: las escribió el desarrollo y van marcadas «sin fuente oficial».

use super::modelo::DesignLang;

/// La palabra de los rangos de un leveler («NIVEL 2-6»). De *Student of
/// Warfare* (ROE) o, en coreano y chino tradicional, de *Hexdrinker* (MH1).
pub fn palabra_nivel(lang: DesignLang) -> &'static str {
    match lang {
        DesignLang::Es => "NIVEL",
        DesignLang::En => "LEVEL",
        DesignLang::Fr => "NIVEAU",
        DesignLang::De => "STUFE",
        DesignLang::It => "LIVELLO",
        DesignLang::Pt => "NÍVEL",
        DesignLang::Ru => "УРОВЕНЬ",
        DesignLang::Ja => "Lv",
        DesignLang::Ko => "레벨",
        DesignLang::Zhs => "等级",
        DesignLang::Zht => "等級",
        // Sin fuente oficial.
        DesignLang::Ro => "NIVEL",
        DesignLang::Pl => "POZIOM",
        DesignLang::Nl => "NIVEAU",
        DesignLang::Uk => "РІВЕНЬ",
    }
}

/// El código de idioma que llevan las cartas impresas en el pie (`{lang}` de
/// la plantilla del pie). Comprobado en el pie de cartas recientes de cada
/// idioma en Scryfall (FDN en francés, alemán, italiano y japonés; CLB en
/// ruso, coreano y chino tradicional; ACR en chino simplificado). El
/// portugués no se pudo ver (Scryfall enseña la imagen inglesa): «PT», como
/// su código. En los idiomas sin cartas, el código ISO.
///
/// 🛑 Las cartas en español ponen «SP», pero aquí sigue «ES», como antes de
/// haber más idiomas: cambiarlo cambiaría todas las cartas españolas hechas.
pub fn codigo_pie(lang: DesignLang) -> &'static str {
    match lang {
        DesignLang::Es => "ES",
        DesignLang::En => "EN",
        DesignLang::Fr => "FR",
        DesignLang::De => "DE",
        DesignLang::It => "IT",
        DesignLang::Pt => "PT",
        DesignLang::Ru => "RU",
        DesignLang::Ja => "JP",
        DesignLang::Ko => "KR",
        DesignLang::Zhs => "CS",
        DesignLang::Zht => "CT",
        DesignLang::Ro => "RO",
        DesignLang::Pl => "PL",
        DesignLang::Nl => "NL",
        DesignLang::Uk => "UK",
    }
}

/// Todas las palabras de nivel, para reconocer los rangos en el texto de
/// reglas de cualquier idioma.
pub fn palabras_nivel() -> Vec<&'static str> {
    let mut v: Vec<&str> = DesignLang::TODOS
        .iter()
        .map(|l| palabra_nivel(*l))
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// La palabra clave de Fusionar con su recordatorio. De *Turn // Burn* (GK1)
/// o *Wear // Tear* (DMC, CMR, PIP).
pub fn recordatorio_fusionar(lang: DesignLang) -> &'static str {
    match lang {
        DesignLang::Es => {
            "Fusionar (Puedes lanzar una o las dos mitades de esta carta desde tu mano.)"
        }
        DesignLang::En => "Fuse (You may cast one or both halves of this card from your hand.)",
        DesignLang::Fr => {
            "Fusion (Vous pouvez lancer une ou deux moitiés de cette carte depuis votre main.)"
        }
        DesignLang::De => {
            "Fusion (Du kannst eine oder beide Hälften dieser Karte aus deiner Hand wirken.)"
        }
        DesignLang::It => {
            "Fusione (Puoi lanciare una o entrambe le metà di questa carta dalla tua mano.)"
        }
        DesignLang::Pt => {
            "Fundir (Você pode conjurar uma ou ambas as metades deste card da sua mão.)"
        }
        DesignLang::Ru => {
            "Слияние (Вы можете разыграть одну или обе половины этой карты из вашей руки.)"
        }
        DesignLang::Ja => {
            "融合（あなたはこのカードの片方の半分または両方の半分をあなたの手札から唱えてもよい。）"
        }
        DesignLang::Zhs => "融咒（你可以从你手上单独施放此牌任一边或一同施放两边。）",
        DesignLang::Zht => "融咒（你可以從你手上單獨施放此牌任一邊或一同施放兩邊。）",
        // Scryfall no tiene cartas con Fusionar en coreano: sin fuente oficial.
        DesignLang::Ko => {
            "융합 (당신은 이 카드의 한쪽 또는 양쪽 절반을 당신의 손에서 발동할 수 있다.)"
        }
        // Sin fuente oficial.
        DesignLang::Ro => {
            "Fuziune (Poți lansa una sau ambele jumătăți ale acestei cărți din mâna ta.)"
        }
        DesignLang::Pl => "Fuzja (Możesz rzucić jedną lub obie połówki tej karty ze swojej ręki.)",
        DesignLang::Nl => {
            "Fusie (Je mag een of beide helften van deze kaart vanuit je hand uitspreken.)"
        }
        DesignLang::Uk => {
            "Злиття (Ви можете розіграти одну або обидві половини цієї карти з вашої руки.)"
        }
    }
}

/// La palabra clave de Fusionar sola (lo que hay antes del recordatorio).
pub fn palabra_fusionar(lang: DesignLang) -> &'static str {
    let r = recordatorio_fusionar(lang);
    let fin = r.find([' ', '（']).unwrap_or(r.len());
    &r[..fin]
}

/// Si una línea de reglas es la de Fusionar, en cualquier idioma.
pub fn es_linea_fusionar(linea: &str) -> bool {
    DesignLang::TODOS
        .iter()
        .any(|l| linea.starts_with(palabra_fusionar(*l)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_palabra_de_fusionar_es_la_primera() {
        assert_eq!(palabra_fusionar(DesignLang::Es), "Fusionar");
        assert_eq!(palabra_fusionar(DesignLang::En), "Fuse");
        assert_eq!(palabra_fusionar(DesignLang::Ja), "融合");
        assert_eq!(palabra_fusionar(DesignLang::Zht), "融咒");
        assert!(es_linea_fusionar("Fusion (Du kannst…)"));
        assert!(es_linea_fusionar("Fusionar. (Puedes…)"));
        assert!(!es_linea_fusionar("Destruye la criatura objetivo."));
    }

    #[test]
    fn el_codigo_del_pie_de_cada_idioma() {
        let codigos: Vec<&str> = DesignLang::TODOS.iter().map(|l| codigo_pie(*l)).collect();
        assert_eq!(
            codigos,
            [
                "ES", "EN", "FR", "DE", "IT", "PT", "RO", "PL", "NL", "RU", "UK", "JP", "KR", "CS",
                "CT"
            ]
        );
    }

    #[test]
    fn todos_los_idiomas_tienen_sus_palabras() {
        for l in DesignLang::TODOS {
            assert!(!palabra_nivel(l).is_empty());
            assert!(recordatorio_fusionar(l).contains(['(', '（']));
        }
        assert!(palabras_nivel().contains(&"STUFE"));
    }
}
