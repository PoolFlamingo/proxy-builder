//! **Líneas de tipo sin red** en los idiomas de diseño (el español tiene su
//! propia gramática en `borrador::tipo_en_espanol`). Evitan que salga «Land»
//! en una carta en polaco cuando Scryfall no trae la línea impresa.
//!
//! Para no inventar una gramática por idioma (géneros, casos), las tablas
//! guardan **frases enteras** por combinación habitual («Legendary Land»,
//! «Basic Land»…) y los subtipos más comunes. Fuera de eso, `None`: manda lo
//! que imprimió Scryfall, o el inglés.
//!
//! En los idiomas con cartas impresas, cada frase sale de una carta real en
//! Scryfall (anotada al lado; el sustantivo de «Planeswalker» solo, de su
//! forma legendaria, porque no hay ninguno sin ella). Rumano, polaco,
//! neerlandés y ucraniano no tienen cartas: sus tablas las escribió el
//! desarrollo y van **sin fuente oficial**.

use super::modelo::DesignLang;

struct Tabla {
    tipos: &'static [(&'static str, &'static str)],
    subtipos: &'static [(&'static str, &'static str)],
    /// Entre los tipos y los subtipos (« — »; el chino, « ～»).
    separador: &'static str,
    /// Entre dos subtipos; `None` si el idioma cambia el orden y solo se
    /// sabe uno («Druido Elfo» en italiano).
    union: Option<&'static str>,
    /// Entre los dos últimos subtipos, si no es la unión (« et » en francés).
    ultima: Option<&'static str>,
}

const FR: Tabla = Tabla {
    tipos: &[
        ("Land", "Terrain"),                           // Abandoned Campground (DSK 255)
        ("Legendary Land", "Terrain légendaire"),      // Academy Ruins (DRC 58)
        ("Creature", "Créature"),                      // Aarakocra Sneak (CLB 54)
        ("Legendary Creature", "Créature légendaire"), // Aatchik, Emerald Radian (DFT 187)
        ("Artifact", "Artefact"),                      // Abandoned Sarcophagus (C20 236)
        ("Legendary Artifact", "Artefact légendaire"), // Ace's Baseball Bat (WHO 170)
        ("Enchantment", "Enchantement"),               // Abiding Grace (MH2 1)
        ("Legendary Enchantment", "Enchantement légendaire"), // The Sibsig Ceremony (TDM 340)
        ("Instant", "Éphémère"),                       // Abandon Reason (J25 513)
        ("Legendary Instant", "Éphémère légendaire"),  // Isildur's Fateful Strike (LTR 356)
        ("Sorcery", "Rituel"),                         // Abandon the Post (MID 127)
        ("Legendary Sorcery", "Rituel légendaire"),    // Urza's Ruinous Blast (CMM 842)
        ("Planeswalker", "Planeswalker"),
        ("Legendary Planeswalker", "Planeswalker légendaire"), // Kaya, Geist Hunter (TDC 294)
        ("Battle", "Bataille"),                                // Invasion of Alara (MOM 230)
        ("Plane", "Plan"),           // Stormcage Containment Facility (WHO 600)
        ("Phenomenon", "Phénomène"), // Fixed Point in Time (WHO 582)
        ("Scheme", "Machination"),   // Your Plans Mean Nothing (DSC 367)
        ("Basic Land", "Terrain de base"), // Forest (TDM 291)
    ],
    subtipos: &[
        ("Plains", "plaine"),        // Plains (TDM 287)
        ("Island", "île"),           // Island (TDM 288)
        ("Swamp", "marais"),         // Swamp (TDM 289)
        ("Mountain", "montagne"),    // Mountain (TDM 290)
        ("Forest", "forêt"),         // Forest (TDM 291)
        ("Equipment", "équipement"), // Adaptive Omnitool (DRC 16)
        ("Vehicle", "véhicule"),     // Cultivator's Caravan (DRC 127)
        ("Treasure", "trésor"),      // Glittering Stockpile (OTC 167)
        ("Clue", "indice"),          // Found Footage (DSK 246)
        ("Food", "nourriture"),      // Heaped Harvest (BLB 175)
        ("Aura", "aura"),            // Shiny Impetus (TDC 231)
        ("Saga", "saga"),            // Thunder of Unity (TDM 391)
        ("Class", "classe"),         // Wizard Class (BLC 112)
        ("Shrine", "reliquaire"),    // Go-Shintai of Boundless Vigor (NEO 187)
        ("Lesson", "leçon"),         // Basic Conjuration (STX 315)
        ("Trap", "piège"),           // Mindbreak Trap (OTP 12)
    ],
    separador: " — ",
    // Llanowar Elves: «Créature — elfe et druide».
    union: Some(", "),
    ultima: Some(" et "),
};

const DE: Tabla = Tabla {
    tipos: &[
        ("Land", "Land"),                              // Abandoned Campground (DSK 255)
        ("Legendary Land", "Legendäres Land"),         // Academy Ruins (DRC 58)
        ("Creature", "Kreatur"),                       // Aarakocra Sneak (CLB 54)
        ("Legendary Creature", "Legendäre Kreatur"),   // Aatchik, Emerald Radian (DFT 187)
        ("Artifact", "Artefakt"),                      // Abandoned Sarcophagus (C20 236)
        ("Legendary Artifact", "Legendäres Artefakt"), // Ace's Baseball Bat (WHO 170)
        ("Enchantment", "Verzauberung"),               // de la legendaria
        ("Legendary Enchantment", "Legendäre Verzauberung"), // The Sibsig Ceremony (TDM 340)
        ("Instant", "Spontanzauber"),                  // Abandon Reason (J25 513)
        ("Legendary Instant", "Legendärer Spontanzauber"), // Isildur's Fateful Strike (LTR 356)
        ("Sorcery", "Hexerei"),                        // Abandon Hope (TMP 107)
        ("Legendary Sorcery", "Legendäre Hexerei"),    // Urza's Ruinous Blast (CMM 842)
        ("Planeswalker", "Planeswalker"),
        ("Legendary Planeswalker", "Legendärer Planeswalker"), // Kaya, Geist Hunter (TDC 294)
        ("Battle", "Schlacht"),                                // Invasion of Alara (MOM 230)
        ("Plane", "Welt"),              // Stormcage Containment Facility (WHO 600)
        ("Phenomenon", "Phänomen"),     // Fixed Point in Time (WHO 582)
        ("Scheme", "Komplott"),         // Your Plans Mean Nothing (DSC 367)
        ("Basic Land", "Standardland"), // Forest (TDM 291)
    ],
    subtipos: &[
        ("Plains", "Ebene"),         // Plains (TDM 287)
        ("Island", "Insel"),         // Island (TDM 288)
        ("Swamp", "Sumpf"),          // Swamp (TDM 289)
        ("Mountain", "Gebirge"),     // Mountain (TDM 290)
        ("Forest", "Wald"),          // Forest (TDM 291)
        ("Equipment", "Ausrüstung"), // Accorder's Shield (M14 204)
        ("Vehicle", "Fahrzeug"),     // Cultivator's Caravan (DRC 127)
        ("Treasure", "Schatz"),      // Glittering Stockpile (OTC 167)
        ("Clue", "Hinweis"),         // Found Footage (DSK 246)
        ("Food", "Speise"),          // Heaped Harvest (BLB 175)
        ("Aura", "Aura"),            // Shiny Impetus (TDC 231)
        ("Saga", "Sage"),            // Thunder of Unity (TDM 391)
        ("Class", "Klasse"),         // Wizard Class (BLC 112)
        ("Shrine", "Schrein"),       // Go-Shintai of Boundless Vigor (NEO 187)
        ("Lesson", "Lektion"),       // Basic Conjuration (STX 315)
        ("Trap", "Falle"),           // Mindbreak Trap (OTP 12)
    ],
    separador: " — ",
    // Llanowar Elves: «Kreatur — Elf, Druide».
    union: Some(", "),
    ultima: None,
};

const IT: Tabla = Tabla {
    tipos: &[
        ("Land", "Terra"),                               // Abandoned Campground (DSK 255)
        ("Legendary Land", "Terra Leggendaria"),         // Academy Ruins (DRC 58)
        ("Creature", "Creatura"),                        // Aarakocra Sneak (CLB 54)
        ("Legendary Creature", "Creatura Leggendaria"),  // Aatchik, Emerald Radian (DFT 187)
        ("Artifact", "Artefatto"),                       // Abandoned Sarcophagus (C20 236)
        ("Legendary Artifact", "Artefatto Leggendario"), // Mox Jasper (TDM 325)
        ("Enchantment", "Incantesimo"),                  // Abiding Grace (MH2 1)
        ("Legendary Enchantment", "Incantesimo Leggendario"), // The Sibsig Ceremony (TDM 340)
        ("Instant", "Istantaneo"),                       // Abandon Reason (J25 513)
        ("Legendary Instant", "Istantaneo Leggendario"), // Isildur's Fateful Strike (LTR 91)
        ("Sorcery", "Stregoneria"),                      // Abandon the Post (MID 127)
        ("Legendary Sorcery", "Stregoneria Leggendaria"), // Yawgmoth's Vile Offering (MOC 271)
        ("Planeswalker", "Planeswalker"),
        ("Legendary Planeswalker", "Planeswalker Leggendario"), // Kaya, Geist Hunter (TDC 294)
        ("Battle", "Battaglia"),                                // Invasion of Alara (MOM 230)
        ("Plane", "Piano"),                                     // Undercity Reaches (MOC 163)
        ("Phenomenon", "Fenomeno"),                             // Spatial Merging (MOC 158)
        ("Scheme", "Intrigo"),                                  // Your Plans Mean Nothing (DSC 367)
        ("Basic Land", "Terra Base"),                           // Forest (TDM 286)
    ],
    subtipos: &[
        ("Plains", "Pianura"),            // Plains (TDM 278)
        ("Island", "Isola"),              // Island (TDM 280)
        ("Swamp", "Palude"),              // Swamp (TDM 282)
        ("Mountain", "Montagna"),         // Mountain (TDM 284)
        ("Forest", "Foresta"),            // Forest (TDM 286)
        ("Equipment", "Equipaggiamento"), // Swiftfoot Boots (TDC 328)
        ("Vehicle", "Veicolo"),           // Cultivator's Caravan (DRC 127)
        ("Treasure", "Tesoro"),           // Buried Treasure (LCI 246)
        ("Clue", "Indizio"),              // Found Footage (DSK 246)
        ("Food", "Cibo"),                 // Heaped Harvest (BLB 175)
        ("Aura", "Aura"),                 // Shiny Impetus (TDC 231)
        ("Saga", "Saga"),                 // Thunder of Unity (TDM 391)
        ("Class", "Classe"),              // Innkeeper's Talent (BLB 180)
        ("Shrine", "Santuario"),          // Go-Shintai of Boundless Vigor (NEO 187)
        ("Lesson", "Lezione"),            // Basic Conjuration (STX 315)
        ("Trap", "Trappola"),             // Mindbreak Trap (OTP 12)
    ],
    separador: " — ",
    // Llanowar Elves: «Creatura — Druido Elfo» (el orden cambia).
    union: None,
    ultima: None,
};

const PT: Tabla = Tabla {
    tipos: &[
        ("Land", "Terreno"),                                // Wooded Foothills (MH3 441)
        ("Legendary Land", "Terreno Lendário"),             // Phyrexian Tower (MH3 355)
        ("Creature", "Criatura"),                           // Grizzled Leotau (PLST ARB-69)
        ("Legendary Creature", "Criatura Lendária"),        // Rosheen, Roaring Prophet (MH3 434)
        ("Artifact", "Artefato"),                           // Unstable Amulet (MH3 421)
        ("Legendary Artifact", "Artefato Lendário"),        // Mindslaver (OTP 63)
        ("Enchantment", "Encantamento"),                    // Primal Prayers (MH3 429)
        ("Legendary Enchantment", "Encantamento Lendário"), // Necrodominance (MH3 411)
        ("Instant", "Mágica Instantânea"),                  // Flusterstorm (MH3 496)
        ("Legendary Instant", "Mágica Instantânea Lendária"), // Isildur's Fateful Strike (LTR 91)
        ("Sorcery", "Feitiço"),                             // Flare of Cultivation (MH3 425)
        ("Legendary Sorcery", "Feitiço Lendário"),          // Yawgmoth's Vile Offering (MOC 271)
        ("Planeswalker", "Planeswalker"),
        ("Legendary Planeswalker", "Planeswalker Lendário"), // Oko, Thief of Crowns (OTP 50)
        ("Battle", "Batalha"),                               // Invasion of Alara (MOM 230)
        ("Plane", "Plano"),                                  // Undercity Reaches (MOC 163)
        ("Phenomenon", "Fenômeno"),                          // Spatial Merging (MOC 158)
        ("Basic Land", "Terreno Básico"),                    // Forest (MH3 319)
    ],
    subtipos: &[
        ("Plains", "Planície"),       // Plains (MH3 311)
        ("Island", "Ilha"),           // Island (MH3 313)
        ("Swamp", "Pântano"),         // Swamp (MH3 315)
        ("Mountain", "Montanha"),     // Mountain (MH3 317)
        ("Forest", "Floresta"),       // Forest (MH3 319)
        ("Equipment", "Equipamento"), // Meteoric Mace (MH3 283)
        ("Vehicle", "Veículo"),       // Dreadmobile (MH3 87)
        ("Treasure", "Tesouro"),      // Buried Treasure (LCI 246)
        ("Clue", "Pista"),            // Esoteric Duplicator (BIG 5)
        ("Food", "Comida"),           // Three Bowls of Porridge (WOE 253)
        ("Aura", "Aura"),             // Lion Umbra (MH3 426)
        ("Saga", "Saga"),             // The Hunger Tide Rises (MH3 158)
        ("Class", "Classe"),          // Artificer Class (CLB 663)
        ("Shrine", "Sacrário"),       // Go-Shintai of Boundless Vigor (NEO 187)
        ("Lesson", "Lição"),          // Spirit Summoning (STX 236)
        ("Trap", "Armadilha"),        // Mindbreak Trap (OTP 12)
    ],
    separador: " — ",
    // Llanowar Elves: «Criatura — Elfo Druida».
    union: Some(" "),
    ultima: None,
};

const RU: Tabla = Tabla {
    tipos: &[
        ("Land", "Земля"),                               // Windbrisk Heights (CLB 930)
        ("Legendary Land", "Легендарная Земля"),         // Kher Keep (CLB 899)
        ("Creature", "Существо"),                        // Elder Brain (CLB 936)
        ("Legendary Creature", "Легендарное Существо"),  // Niv-Mizzet, Parun (CLB 851)
        ("Artifact", "Артефакт"),                        // Thought Vessel (CLB 879)
        ("Legendary Artifact", "Легендарный Артефакт"),  // Luxior, Giada's Gift (SNC 240)
        ("Enchantment", "Чары"),                         // Firja's Retribution (CLB 846)
        ("Legendary Enchantment", "Легендарные Чары"),   // Passionate Archaeologist (CLB 656)
        ("Instant", "Мгновенное заклинание"),            // Memory Plunder (CLB 849)
        ("Sorcery", "Волшебство"),                       // Extract from Darkness (CLB 844)
        ("Legendary Sorcery", "Легендарное Волшебство"), // Jaya's Immolating Inferno (CMR 415)
        ("Planeswalker", "Planeswalker"),
        ("Legendary Planeswalker", "Легендарный Planeswalker"), // Xenagos, the Reveler (CLB 853)
        ("Basic Land", "Базовая Земля"),                        // Forest (CLB 470)
    ],
    subtipos: &[
        ("Plains", "Равнина"),       // Plains (CLB 454)
        ("Island", "Остров"),        // Island (CLB 458)
        ("Swamp", "Болото"),         // Swamp (CLB 462)
        ("Mountain", "Гора"),        // Mountain (CLB 466)
        ("Forest", "Лес"),           // Forest (CLB 470)
        ("Equipment", "Снаряжение"), // Skullclamp (CLB 870)
        ("Vehicle", "Машина"),       // Nautiloid Ship (CLB 373)
        ("Treasure", "Клад"),        // Dire Mimic (CLB 310)
        ("Clue", "Улика"),           // Agent's Toolkit (NCC 66)
        ("Food", "Еда"),             // Golden Egg (ELD 220)
        ("Aura", "Аура"),            // Endless Evil (CLB 665)
        ("Saga", "Сага"),            // Firja's Retribution (CLB 846)
        ("Class", "Класс"),          // Artificer Class (CLB 663)
        ("Shrine", "Святыня"),       // Go-Shintai of Boundless Vigor (NEO 187)
        ("Lesson", "Урок"),          // Spirit Summoning (STX 236)
        ("Trap", "Ловушка"),         // Whiplash Trap (C20 127)
    ],
    separador: " — ",
    // Llanowar Elves: «Существо — Эльф Друид».
    union: Some(" "),
    ultima: None,
};

const JA: Tabla = Tabla {
    tipos: &[
        ("Land", "土地"),                                 // Abandoned Campground (DSK 255)
        ("Legendary Land", "伝説の土地"),                 // Academy Ruins (DRC 58)
        ("Creature", "クリーチャー"),                     // Aarakocra Sneak (CLB 54)
        ("Legendary Creature", "伝説のクリーチャー"),     // Aatchik, Emerald Radian (DFT 187)
        ("Artifact", "アーティファクト"),                 // Abandoned Sarcophagus (C20 236)
        ("Legendary Artifact", "伝説のアーティファクト"), // Ace's Baseball Bat (WHO 170)
        ("Enchantment", "エンチャント"),                  // Abiding Grace (MH2 1)
        ("Legendary Enchantment", "伝説のエンチャント"),  // The Sibsig Ceremony (TDM 340)
        ("Instant", "インスタント"),                      // Abandon Reason (J25 513)
        ("Legendary Instant", "伝説のインスタント"),      // Isildur's Fateful Strike (LTR 356)
        ("Sorcery", "ソーサリー"),                        // Strategic Betrayal (PWCS 2026-6)
        ("Legendary Sorcery", "伝説のソーサリー"),        // Urza's Ruinous Blast (CMM 842)
        ("Planeswalker", "プレインズウォーカー"),
        ("Legendary Planeswalker", "伝説のプレインズウォーカー"), // The Eternal Wanderer (PW26 4)
        ("Battle", "バトル"),                                     // Invasion of Alara (MOM 230)
        ("Plane", "次元"),          // Stormcage Containment Facility (WHO 600)
        ("Phenomenon", "現象"),     // Fixed Point in Time (WHO 582)
        ("Scheme", "計略"),         // Your Plans Mean Nothing (DSC 367)
        ("Basic Land", "基本土地"), // Forest (TDM 291)
    ],
    subtipos: &[
        ("Plains", "平地"),      // Plains (TDM 287)
        ("Island", "島"),        // Island (TDM 288)
        ("Swamp", "沼"),         // Swamp (TDM 289)
        ("Mountain", "山"),      // Mountain (TDM 290)
        ("Forest", "森"),        // Forest (TDM 291)
        ("Equipment", "装備品"), // Accorder's Shield (M14 204)
        ("Vehicle", "機体"),     // Cultivator's Caravan (DRC 127)
        ("Treasure", "宝物"),    // Glittering Stockpile (OTC 167)
        ("Clue", "手掛かり"),    // Found Footage (DSK 246)
        ("Food", "食物"),        // Heaped Harvest (BLB 175)
        ("Aura", "オーラ"),      // Sheltered by Ghosts (PJSC 2026-1)
        ("Saga", "英雄譚"),      // Thunder of Unity (TDM 391)
        ("Class", "クラス"),     // Wizard Class (BLC 112)
        ("Shrine", "祭殿"),      // Go-Shintai of Boundless Vigor (NEO 187)
        ("Lesson", "講義"),      // Basic Conjuration (STX 315)
        ("Trap", "罠"),          // Mindbreak Trap (OTP 12)
    ],
    separador: " — ",
    // Llanowar Elves: «クリーチャー — エルフ・ドルイド».
    union: Some("・"),
    ultima: None,
};

const KO: Tabla = Tabla {
    tipos: &[
        ("Land", "대지"),                             // Spire Garden (CLB 361)
        ("Legendary Land", "전설적 대지"),            // Baldur's Gate (CLB 345)
        ("Creature", "생물"),                         // Elder Brain (CLB 936)
        ("Legendary Creature", "전설적 생물"),        // Zevlor, Elturel Exile (CLB 441)
        ("Artifact", "마법물체"),                     // Stonespeaker Crystal (CLB 450)
        ("Legendary Artifact", "전설적 마법물체"),    // Luxior, Giada's Gift (SNC 240)
        ("Enchantment", "부여마법"),                  // Legion Loyalty (CLB 371)
        ("Legendary Enchantment", "전설적 부여마법"), // Raised by Giants (CLB 250)
        ("Instant", "순간마법"),                      // Lightning Bolt (CLB 401)
        ("Sorcery", "집중마법"),                      // Fireball (CLB 397)
        ("Legendary Sorcery", "전설적 집중마법"),     // Primevals' Glorious Rebirth (DOM 201)
        ("Planeswalker", "플레인즈워커"),
        ("Legendary Planeswalker", "전설적 플레인즈워커"), // Tasha, the Witch Queen (CLB 364)
        ("Basic Land", "기본 대지"),                       // Forest (CLB 470)
    ],
    subtipos: &[
        ("Plains", "들"),      // Plains (CLB 454)
        ("Island", "섬"),      // Island (CLB 458)
        ("Swamp", "늪"),       // Swamp (CLB 462)
        ("Mountain", "산"),    // Mountain (CLB 466)
        ("Forest", "숲"),      // Forest (CLB 470)
        ("Equipment", "장비"), // Cloak of the Bat (CLB 443)
        ("Vehicle", "탑승물"), // Nautiloid Ship (CLB 373)
        ("Treasure", "보물"),  // Dire Mimic (CLB 310)
        ("Food", "음식"),      // Golden Egg (ELD 220)
        ("Aura", "마법진"),    // Predatory Impetus (CLB 249)
        ("Saga", "서사시"),    // Waking the Trolls (KHM 234)
        ("Class", "직업"),     // Sorcerer Class (AFR 233)
        ("Shrine", "성지"),    // Go-Shintai of Boundless Vigor (NEO 187)
        ("Lesson", "수업"),    // Spirit Summoning (STX 236)
    ],
    separador: " — ",
    // Llanowar Elves: «생물 — 엘프 드루이드».
    union: Some(" "),
    ultima: None,
};

const ZHS: Tabla = Tabla {
    tipos: &[
        ("Land", "地"),                        // Brotherhood Headquarters (ACR 266)
        ("Legendary Land", "传奇地"),          // Abstergo Entertainment (ACR 265)
        ("Creature", "生物"),                  // Palazzo Archers (ACR 222)
        ("Legendary Creature", "传奇生物"),    // Sokrates, Athenian Teacher (ACR 250)
        ("Artifact", "神器"),                  // Hidden Blade (ACR 307)
        ("Legendary Artifact", "传奇神器"),    // Yggdrasil, Rebirth Engine (ACR 264)
        ("Enchantment", "结界"),               // Bleeding Effect (ACR 232)
        ("Legendary Enchantment", "传奇结界"), // Necrodominance (MH3 411)
        ("Instant", "瞬间"),                   // Terminate (ACR 251)
        ("Legendary Instant", "传奇瞬间"),     // Isildur's Fateful Strike (LTR 356)
        ("Sorcery", "法术"),                   // Reconstruct History (ACR 245)
        ("Legendary Sorcery", "传奇法术"),     // Yawgmoth's Vile Offering (MOC 271)
        ("Planeswalker", "鹏洛客"),
        ("Legendary Planeswalker", "传奇鹏洛客"), // Oko, Thief of Crowns (OTP 50)
        ("Battle", "战役"),                       // Invasion of Alara (MOM 230)
        ("Plane", "时空"),                        // Undercity Reaches (MOC 163)
        ("Phenomenon", "异象"),                   // Spatial Merging (MOC 158)
        ("Basic Land", "基本地"),                 // Forest (ACR 110)
    ],
    subtipos: &[
        ("Plains", "平原"),    // Plains (ACR 102)
        ("Island", "海岛"),    // Island (ACR 104)
        ("Swamp", "沼泽"),     // Swamp (ACR 106)
        ("Mountain", "山脉"),  // Mountain (ACR 108)
        ("Forest", "树林"),    // Forest (ACR 110)
        ("Equipment", "武具"), // Hidden Blade (ACR 307)
        ("Vehicle", "载具"),   // Adrestia (ACR 252)
        ("Treasure", "珍宝"),  // Glittering Stockpile (OTC 167)
        ("Clue", "线索"),      // Esoteric Duplicator (BIG 70)
        ("Food", "食品"),      // Three Bowls of Porridge (WOE 253)
        ("Aura", "灵气"),      // Amphibian Downpour (MH3 449)
        ("Saga", "传纪"),      // The Aesir Escape Valhalla (ACR 39)
        ("Class", "职业"),     // Artificer Class (CLB 663)
        ("Shrine", "祭祠"),    // Go-Shintai of Boundless Vigor (NEO 187)
        ("Lesson", "课程"),    // Spirit Summoning (STX 236)
        ("Trap", "陷阱"),      // Mindbreak Trap (OTP 12)
    ],
    // Llanowar Elves: «生物 ～妖精／德鲁伊».
    separador: " ～",
    union: Some("／"),
    ultima: None,
};

const ZHT: Tabla = Tabla {
    tipos: &[
        ("Land", "地"),                        // Windbrisk Heights (CLB 930)
        ("Legendary Land", "傳奇地"),          // Kher Keep (CLB 899)
        ("Creature", "生物"),                  // Elder Brain (CLB 936)
        ("Legendary Creature", "傳奇生物"),    // Niv-Mizzet, Parun (CLB 851)
        ("Artifact", "神器"),                  // Thought Vessel (CLB 879)
        ("Legendary Artifact", "傳奇神器"),    // Luxior, Giada's Gift (SNC 240)
        ("Enchantment", "結界"),               // Firja's Retribution (CLB 846)
        ("Legendary Enchantment", "傳奇結界"), // Passionate Archaeologist (CLB 656)
        ("Instant", "瞬間"),                   // Memory Plunder (CLB 849)
        ("Sorcery", "巫術"),                   // Extract from Darkness (CLB 844)
        ("Legendary Sorcery", "傳奇巫術"),     // Jaya's Immolating Inferno (CMR 415)
        ("Planeswalker", "鵬洛客"),
        ("Legendary Planeswalker", "傳奇鵬洛客"), // Xenagos, the Reveler (CLB 853)
        ("Basic Land", "基本地"),                 // Forest (CLB 470)
    ],
    subtipos: &[
        ("Plains", "平原"),    // Plains (CLB 454)
        ("Island", "海島"),    // Island (CLB 458)
        ("Swamp", "沼澤"),     // Swamp (CLB 462)
        ("Mountain", "山脈"),  // Mountain (CLB 466)
        ("Forest", "樹林"),    // Forest (CLB 470)
        ("Equipment", "武具"), // Skullclamp (CLB 870)
        ("Vehicle", "載具"),   // Nautiloid Ship (CLB 373)
        ("Treasure", "珍寶"),  // Dire Mimic (CLB 310)
        ("Clue", "線索"),      // Agent's Toolkit (NCC 66)
        ("Food", "食品"),      // Golden Egg (ELD 220)
        ("Aura", "靈氣"),      // Endless Evil (CLB 665)
        ("Saga", "傳紀"),      // Firja's Retribution (CLB 846)
        ("Class", "職業"),     // Artificer Class (CLB 663)
        ("Shrine", "祭祠"),    // Go-Shintai of Boundless Vigor (NEO 187)
        ("Lesson", "課程"),    // Spirit Summoning (STX 236)
    ],
    // Llanowar Elves: «生物 ～妖精／德魯伊».
    separador: " ～",
    union: Some("／"),
    ultima: None,
};

// ── Sin fuente oficial (no hay cartas impresas en estos idiomas) ───────────

const RO: Tabla = Tabla {
    tipos: &[
        ("Land", "Teren"),
        ("Legendary Land", "Teren legendar"),
        ("Creature", "Creatură"),
        ("Legendary Creature", "Creatură legendară"),
        ("Artifact", "Artefact"),
        ("Legendary Artifact", "Artefact legendar"),
        ("Enchantment", "Farmec"),
        ("Legendary Enchantment", "Farmec legendar"),
        ("Instant", "Instantaneu"),
        ("Legendary Instant", "Instantaneu legendar"),
        ("Sorcery", "Vrajă"),
        ("Legendary Sorcery", "Vrajă legendară"),
        ("Planeswalker", "Planeswalker"),
        ("Legendary Planeswalker", "Planeswalker legendar"),
        ("Battle", "Bătălie"),
        ("Plane", "Plan"),
        ("Phenomenon", "Fenomen"),
        ("Scheme", "Uneltire"),
        ("Basic Land", "Teren de bază"),
    ],
    subtipos: &[
        ("Plains", "Câmpie"),
        ("Island", "Insulă"),
        ("Swamp", "Mlaștină"),
        ("Mountain", "Munte"),
        ("Forest", "Pădure"),
        ("Equipment", "Echipament"),
        ("Vehicle", "Vehicul"),
        ("Treasure", "Comoară"),
        ("Clue", "Indiciu"),
        ("Food", "Hrană"),
        ("Blood", "Sânge"),
        ("Gold", "Aur"),
        ("Aura", "Aură"),
        ("Saga", "Saga"),
        ("Class", "Clasă"),
        ("Curse", "Blestem"),
        ("Shrine", "Altar"),
        ("Lesson", "Lecție"),
        ("Trap", "Capcană"),
    ],
    separador: " — ",
    union: Some(" "),
    ultima: None,
};

const PL: Tabla = Tabla {
    tipos: &[
        ("Land", "Kraina"),
        ("Legendary Land", "Legendarna kraina"),
        ("Creature", "Stwór"),
        ("Legendary Creature", "Legendarny stwór"),
        ("Artifact", "Artefakt"),
        ("Legendary Artifact", "Legendarny artefakt"),
        ("Enchantment", "Zaklęcie"),
        ("Legendary Enchantment", "Legendarne zaklęcie"),
        ("Instant", "Czar natychmiastowy"),
        ("Legendary Instant", "Legendarny czar natychmiastowy"),
        ("Sorcery", "Czar"),
        ("Legendary Sorcery", "Legendarny czar"),
        ("Planeswalker", "Planeswalker"),
        ("Legendary Planeswalker", "Legendarny planeswalker"),
        ("Battle", "Bitwa"),
        ("Plane", "Płaszczyzna"),
        ("Phenomenon", "Zjawisko"),
        ("Scheme", "Intryga"),
        ("Basic Land", "Podstawowa kraina"),
    ],
    subtipos: &[
        ("Plains", "Równina"),
        ("Island", "Wyspa"),
        ("Swamp", "Bagno"),
        ("Mountain", "Góra"),
        ("Forest", "Las"),
        ("Equipment", "Ekwipunek"),
        ("Vehicle", "Pojazd"),
        ("Treasure", "Skarb"),
        ("Clue", "Trop"),
        ("Food", "Jedzenie"),
        ("Blood", "Krew"),
        ("Gold", "Złoto"),
        ("Aura", "Aura"),
        ("Saga", "Saga"),
        ("Class", "Klasa"),
        ("Curse", "Klątwa"),
        ("Shrine", "Kapliczka"),
        ("Lesson", "Lekcja"),
        ("Trap", "Pułapka"),
    ],
    separador: " — ",
    union: Some(" "),
    ultima: None,
};

const NL: Tabla = Tabla {
    tipos: &[
        ("Land", "Land"),
        ("Legendary Land", "Legendarisch land"),
        ("Creature", "Wezen"),
        ("Legendary Creature", "Legendarisch wezen"),
        ("Artifact", "Artefact"),
        ("Legendary Artifact", "Legendarisch artefact"),
        ("Enchantment", "Betovering"),
        ("Legendary Enchantment", "Legendarische betovering"),
        ("Instant", "Instant"),
        ("Legendary Instant", "Legendarische instant"),
        ("Sorcery", "Tovenarij"),
        ("Legendary Sorcery", "Legendarische tovenarij"),
        ("Planeswalker", "Planeswalker"),
        ("Legendary Planeswalker", "Legendarische planeswalker"),
        ("Battle", "Strijd"),
        ("Plane", "Vlak"),
        ("Phenomenon", "Fenomeen"),
        ("Scheme", "Komplot"),
        ("Basic Land", "Basisland"),
    ],
    subtipos: &[
        ("Plains", "Vlakte"),
        ("Island", "Eiland"),
        ("Swamp", "Moeras"),
        ("Mountain", "Berg"),
        ("Forest", "Bos"),
        ("Equipment", "Uitrusting"),
        ("Vehicle", "Voertuig"),
        ("Treasure", "Schat"),
        ("Clue", "Aanwijzing"),
        ("Food", "Voedsel"),
        ("Blood", "Bloed"),
        ("Gold", "Goud"),
        ("Aura", "Aura"),
        ("Saga", "Saga"),
        ("Class", "Klasse"),
        ("Curse", "Vloek"),
        ("Shrine", "Heiligdom"),
        ("Lesson", "Les"),
        ("Trap", "Val"),
    ],
    separador: " — ",
    union: Some(" "),
    ultima: None,
};

const UK: Tabla = Tabla {
    tipos: &[
        ("Land", "Земля"),
        ("Legendary Land", "Легендарна Земля"),
        ("Creature", "Істота"),
        ("Legendary Creature", "Легендарна Істота"),
        ("Artifact", "Артефакт"),
        ("Legendary Artifact", "Легендарний Артефакт"),
        ("Enchantment", "Чари"),
        ("Legendary Enchantment", "Легендарні Чари"),
        ("Instant", "Миттєве закляття"),
        ("Legendary Instant", "Легендарне Миттєве закляття"),
        ("Sorcery", "Чаклунство"),
        ("Legendary Sorcery", "Легендарне Чаклунство"),
        ("Planeswalker", "Planeswalker"),
        ("Legendary Planeswalker", "Легендарний Planeswalker"),
        ("Battle", "Битва"),
        ("Plane", "Площина"),
        ("Phenomenon", "Феномен"),
        ("Scheme", "Задум"),
        ("Basic Land", "Базова Земля"),
    ],
    subtipos: &[
        ("Plains", "Рівнина"),
        ("Island", "Острів"),
        ("Swamp", "Болото"),
        ("Mountain", "Гора"),
        ("Forest", "Ліс"),
        ("Equipment", "Спорядження"),
        ("Vehicle", "Машина"),
        ("Treasure", "Скарб"),
        ("Clue", "Доказ"),
        ("Food", "Їжа"),
        ("Blood", "Кров"),
        ("Gold", "Золото"),
        ("Aura", "Аура"),
        ("Saga", "Сага"),
        ("Class", "Клас"),
        ("Curse", "Прокляття"),
        ("Shrine", "Святиня"),
        ("Lesson", "Урок"),
        ("Trap", "Пастка"),
    ],
    separador: " — ",
    union: Some(" "),
    ultima: None,
};

fn tabla(lang: DesignLang) -> Option<&'static Tabla> {
    Some(match lang {
        DesignLang::Fr => &FR,
        DesignLang::De => &DE,
        DesignLang::It => &IT,
        DesignLang::Pt => &PT,
        DesignLang::Ru => &RU,
        DesignLang::Ja => &JA,
        DesignLang::Ko => &KO,
        DesignLang::Zhs => &ZHS,
        DesignLang::Zht => &ZHT,
        DesignLang::Ro => &RO,
        DesignLang::Pl => &PL,
        DesignLang::Nl => &NL,
        DesignLang::Uk => &UK,
        DesignLang::Es | DesignLang::En => return None,
    })
}

/// La línea de tipo en `lang` (ni español ni inglés) de una en inglés, sin
/// red: una combinación conocida de tipos y subtipos conocidos. `None` si
/// algo no se sabe.
pub fn tipo_en(en: &str, lang: DesignLang) -> Option<String> {
    let t = tabla(lang)?;
    let (tipos, subtipos) = match en.split_once('—') {
        Some((a, b)) => (a.trim(), Some(b.trim())),
        None => (en.trim(), None),
    };
    let tipos = tipos.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut r = t.tipos.iter().find(|(k, _)| *k == tipos)?.1.to_string();
    if let Some(sub) = subtipos {
        let palabras: Option<Vec<&str>> = sub
            .split_whitespace()
            .map(|p| t.subtipos.iter().find(|(k, _)| *k == p).map(|(_, v)| *v))
            .collect();
        let palabras = palabras?;
        let n = palabras.len();
        let unidos = match (n, t.union) {
            (0, _) => return None,
            (1, _) => palabras[0].to_string(),
            (_, None) => return None,
            (_, Some(u)) => match t.ultima {
                Some(y) => format!("{}{y}{}", palabras[..n - 1].join(u), palabras[n - 1]),
                None => palabras.join(u),
            },
        };
        r.push_str(t.separador);
        r.push_str(&unidos);
    }
    Some(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_idioma_traduce_lo_basico() {
        for l in DesignLang::TODOS {
            if matches!(l, DesignLang::Es | DesignLang::En) {
                assert!(tabla(l).is_none());
                continue;
            }
            let t = tabla(l).unwrap();
            // Las combinaciones que no pueden faltar en ninguno.
            for k in [
                "Land",
                "Legendary Land",
                "Creature",
                "Legendary Creature",
                "Artifact",
                "Enchantment",
                "Instant",
                "Sorcery",
                "Legendary Planeswalker",
                "Basic Land",
            ] {
                assert!(t.tipos.iter().any(|(x, _)| *x == k), "{l:?}: falta «{k}»");
            }
            for k in [
                "Plains",
                "Island",
                "Swamp",
                "Mountain",
                "Forest",
                "Equipment",
                "Aura",
            ] {
                assert!(
                    t.subtipos.iter().any(|(x, _)| *x == k),
                    "{l:?}: falta «{k}»"
                );
            }
            assert!(tipo_en("Basic Land — Forest", l).is_some(), "{l:?}");
        }
    }

    #[test]
    fn frases_y_subtipos() {
        assert_eq!(
            tipo_en("Basic Land — Forest", DesignLang::De).as_deref(),
            Some("Standardland — Wald")
        );
        assert_eq!(
            tipo_en("Legendary Artifact — Equipment", DesignLang::Ja).as_deref(),
            Some("伝説のアーティファクト — 装備品")
        );
        assert_eq!(
            tipo_en("Land — Island Swamp", DesignLang::Fr).as_deref(),
            Some("Terrain — île et marais")
        );
        assert_eq!(
            tipo_en("Land — Island Swamp", DesignLang::De).as_deref(),
            Some("Land — Insel, Sumpf")
        );
        assert_eq!(
            tipo_en("Land — Plains Forest", DesignLang::Zhs).as_deref(),
            Some("地 ～平原／树林")
        );
        // El italiano cambia el orden de los subtipos: con dos, no se sabe.
        assert_eq!(tipo_en("Land — Island Swamp", DesignLang::It), None);
        assert_eq!(tipo_en("Sorcery", DesignLang::Pl).as_deref(), Some("Czar"));
        // Criaturas (demasiados subtipos) y mezclas, no.
        assert_eq!(tipo_en("Creature — Elf Druid", DesignLang::Fr), None);
        assert_eq!(tipo_en("Artifact Creature — Golem", DesignLang::Ru), None);
        assert_eq!(tipo_en("Land", DesignLang::En), None);
    }
}
