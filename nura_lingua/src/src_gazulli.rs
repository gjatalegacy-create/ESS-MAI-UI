//! Importuesi i fjalorit të Nikollë Gazullit, «Fjalorth i ri – Fjalë të rralla të përdoruna në
//! Veri të Shqipnis» (1941; domen publik sipas afatit). TSV nga `nura_fetch`:
//! `fjala ⇥ shënimi_gramatikor ⇥ shpjegimi`.
//!
//! Teksti vjen nga OCR automatik, prandaj merret vetëm shpjegimi që kalon filtrin e cilësisë:
//! - pa shenja të prishura nga OCR-ja (shifra, «£», «$», thonjëza, «|» …) dhe pa copa të njohura
//!   të prishura («dsht», «kd», «md» …);
//! - shembulli me vetë fjalën («më janë agullue sytë») nuk është shpjegim: merret pjesa pas «:»
//!   ose «;» që nuk e përmend fjalën;
//! - 1–18 fjalë, të paktën një e treta kuptimplota; nuk nis me shënim gramatikor («-a», «s.»,
//!   «fj.», «e palakueshme») ose me shënim të shtojcës («shto», «ndreq»); nuk mbaron me fjalë
//!   funksionale (OCR-ja e preu);
//! - asnjë fjalë me shkronjë të madhe në mes, përveç emrave të vendeve të njohur (ndryshe është
//!   emër latin bime ose copë e huaj);
//! - ≥ 80% e fjalëve janë fjalë shqipe të njohura nga burimet e tjera ose nga vetë fjalori (për
//!   gegërishten e 1941 kontrolli provon edhe trajtën standarde: «-un» → «-ur», «-ue» → «-uar»/«-oj»,
//!   «nji» → «një», mbaresat e shquara);
//! - asnjë copë 1–2 shkronjëshe që s'është fjalë funksionale (shenjë tipike e OCR-së së prishur).
//!
//! Shpjegimi ruhet ashtu siç e shkroi autori, si atribut `gloss` i vetë-lakut
//! `Rel::Sem("REGIONAL_GLOSS")` të fjalës (me `region=veri`), jo si kuptim WordNet dhe jo si të dhënë
//! gramatikore (`Rel::Lex`): pjesa e ligjëratës e Gazullit nuk ndikon te gramatika e fjalëve të
//! zakonshme. Kuptimi krahinor thuhet si përkufizim vetëm kur fjala s'ka kuptim tjetër; ndryshe
//! thuhet pas tij, si shtesë.
use crate::graph::*;
use crate::import::ImportReport;
use crate::morph::Morph;
use std::collections::{BTreeMap, HashSet};

pub const TSV: &str = "raw/gazulli1941/gazulli1941-sq.tsv";
pub const REL: &str = "REGIONAL_GLOSS";

/// Fjalë funksionale shqipe (edhe në trajtën gege të 1941) që lejohen si copa të shkurtra.
const SHORT_OK: &[&str] = &[
    "e", "i", "a", "o", "u", "me", "në", "ne", "të", "te", "qi", "që", "si", "se", "ka", "kë", "ku", "jo", "mos", "pa",
    "për", "per", "mbë", "mbi", "nën", "nji", "një", "ma", "mâ", "më", "ti", "ai", "ky", "kjo", "ajo", "sa", "kur", "dhe",
    "edhe", "edhë", "s'", "m'u", "nga", "prej", "ndër", "tue", "asht", "ishte", "janë", "jam", "kam", "kanë", "atë", "at",
    "ash", "gja", "gjë", "do", "po", "na", "ju", "ia", "ja", "un", "unë", "bë", "rë", "zë", "dy", "tre", "lë", "shi", "ujë",
    "së", "uj", "vi", "vë", "ri", "zi",
];

/// Copa që OCR-ja i prodhon shpesh në këtë skanim (fjalë të prishura, jo fjalë shqipe).
const OCR_TOKENS: &[&str] = &[
    "dsht", "dntue", "kd", "rd", "pd", "md", "fhojcë", "ië", "jë", "j£", "sjt", "tja", "smë", "stë", "thën", "ëstht", "isht",
    "ime", "mënd",
];

/// Emrat e përveçëm që lejohen me shkronjë të madhe në mes të shpjegimit.
const PROPER: &[&str] = &[
    "Shkodër", "Shkodra", "Shkodrës", "Drin", "Drini", "Drinit", "Bunë", "Buna", "Bunës", "Kosovë", "Kosova", "Kosovës",
    "Mirditë", "Mirdita", "Mirditës", "Dukagjin", "Dukagjini", "Dukagjinit", "Malësi", "Malësia", "Malësisë", "Malsi",
    "Malsis", "Zadrimë", "Zadrima", "Zadrimës", "Shqipni", "Shqipnia", "Shqipnis", "Shqipnisë", "Shqipëri", "Shqipërisë",
    "Lezhë", "Lezha", "Lezhës", "Pukë", "Puka", "Pukës", "Tiranë", "Tirana", "Tiranës", "Kelmend", "Kelmendi",
    "Kelmendit", "Gjakovë", "Gjakova", "Gjakovës", "Prizren", "Prizreni", "Prizrenit", "Durrës", "Durrësit", "Krujë",
    "Kruja", "Krujës", "Mat", "Matit", "Has", "Hasit", "Zot", "Zoti", "Zotit", "Krisht", "Krishti", "Krishtit", "Shën",
];

fn ocr_clean(gloss: &str) -> bool {
    gloss.chars().all(|c| c.is_alphabetic() || matches!(c, ' ' | ',' | '.' | ';' | ':' | '\'' | '’' | '-'))
        && gloss.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_ascii_alphabetic() || matches!(c, 'ë' | 'ç' | 'Ë' | 'Ç' | 'â' | 'ê' | 'î' | 'ô' | 'û'))
}

/// Trajtat standarde të mundshme të një fjale gege për kontrollin e leksikut (teksti nuk ndryshon).
fn standard_candidates(t: &str) -> Vec<String> {
    let base = t.replace('â', "a").replace('ê', "e").replace('î', "i").replace('ô', "o").replace('û', "u");
    let mut out = vec![t.to_string(), base.clone()];
    let ends: [(&str, &[&str]); 6] = [
        ("un", &["ur"]),
        ("ue", &["uar", "oj"]),
        ("uem", &["uar"]),
        ("ë", &["", "e"]),
        ("ën", &["ër"]),
        ("ue", &["ues"]),
    ];
    for (end, repls) in ends {
        match base.strip_suffix(end) {
            Some(stem) => {
                for r in repls {
                    out.push(format!("{stem}{r}"));
                }
            }
            None => {}
        }
    }
    match base.as_str() {
        "nji" => out.push("një".into()),
        "qi" => out.push("që".into()),
        "asht" => out.push("është".into()),
        "kë" => out.push("ka".into()),
        _ => {}
    }
    // Trajta e shquar ose e lakuar («liqenit», «malit», «Shkodrës») kontrollohet edhe te lema e
    // mundshme: rregullat e morfologjisë dhe mbaresat e gjinores/dhanores.
    let oblique: [(&str, &str); 10] = [
        ("it", ""),
        ("ut", ""),
        ("ës", "ë"),
        ("ës", ""),
        ("së", ""),
        ("ve", ""),
        ("ët", "ë"),
        ("ët", ""),
        ("ën", "ë"),
        ("ën", ""),
    ];
    let mut stripped: Vec<String> = out.iter().flat_map(|c| crate::morph::strip_candidates(c)).collect();
    for c in out.iter() {
        for (end, r) in oblique {
            match c.strip_suffix(end).filter(|st| st.chars().count() >= 2) {
                Some(stem) => stripped.push(format!("{stem}{r}")),
                None => {}
            }
        }
    }
    out.extend(stripped);
    out
}

/// Heq shkurtimet gramatikore në fillim («adv.», «ady.», «prep.», «akt», «pës.») dhe një copë
/// të shkurtër pas pikës së fundit («… liqënit të Shkodrës. I»).
fn trim_edges(gloss: &str) -> String {
    let mut toks: Vec<&str> = gloss.split_whitespace().collect();
    loop {
        let abbrev = match toks.first().map(|t| t.trim_start_matches(['\'', '’', '‘', '"', '“'])) {
            // «sh.» (shih) dhe «krh.» janë referenca, jo shkurtime gramatikore: mbeten.
            Some(t) => {
                !matches!(t.to_lowercase().trim_end_matches(',').as_ref(), "sh." | "krh." | "kr.")
                    && ((t.ends_with('.') && t.chars().count() <= 6)
                        // Shkurtim gramatikor i prishur me pika brenda («m.j,», «s.f,»).
                        || (t.contains('.') && t.split('.').all(|p| p.trim_matches(',').chars().count() <= 2))
                        || matches!(t, "akt" | "akt," | "pës" | "pës," | "jokal" | "kal"))
            }
            None => false,
        };
        match abbrev {
            true => {
                toks.remove(0);
            }
            false => break,
        }
    }
    let joined = toks.join(" ");
    // Pas pikës: një copë e shkurtër ose një hyrje tjetër e ngjitur nga OCR-ja («… me barrë.
    // bardhë-a») nuk i përket shpjegimit.
    match joined.find(". ") {
        Some(i) => {
            let tail: Vec<&str> = joined[i + 2..].split_whitespace().collect();
            let stray = tail.len() <= 2 || tail.first().map(|t| t.contains('-')).unwrap_or(false);
            match stray {
                true => joined[..i + 1].to_string(),
                false => joined,
            }
        }
        None => joined,
    }
}

/// Mbaresat e shquara dhe të lakimit që Gazulli shënon pas vizës.
const GLOSS_ENDINGS: &[&str] = &[
    "i", "a", "ja", "e", "ni", "u", "t", "ët", "ore", "ori", "ë", "ri", "ia", "it", "ut", "në", "ve", "elli", "olli", "ëlli",
    "oni", "uri", "ta", "na", "ëja", "ëri", "ii", "oili", "ëni", "ëzi",
];

/// Mbaresa e shquar pas vizës («muzg-u», «avull-i», «kapërcyell-elli») hiqet nga çdo fjalë.
fn drop_definite_endings(g: &str) -> String {
    g.split(' ')
        .map(|t| {
            let (core, punct) = match t.strip_suffix([',', ';', ':', '.']) {
                Some(c) => (c, &t[c.len()..]),
                None => (t, ""),
            };
            // Vetëm mbaresat e njohura; pjesa e dytë e fjalës së përbërë mbetet («i pa-matun»,
            // «flok-math»).
            match core.split_once('-') {
                Some((w, end)) => match GLOSS_ENDINGS.contains(&end) {
                    true => format!("{w}{punct}"),
                    false => t.to_string(),
                },
                None => t.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Fjalë funksionale me të cilat një shpjegim i plotë nuk mbaron (shenjë se OCR-ja e preu).
const DANGLING: &[&str] = &[
    "e", "i", "të", "me", "se", "qi", "që", "në", "nji", "një", "a", "kë", "mbë", "për", "ndër", "prej", "tue", "si", "kur",
    "pa", "mbi", "nën", "nga", "ndaj", "dhe", "edhe", "edhë", "ose", "apo", "ma", "mâ", "më", "s'", "kah", "te", "tek",
    "deri", "dëri", "mos",
];

/// Fjalët e shpjegimit: pa pikësim dhe me shkronja të vogla. `split_hyphen` heq mbaresën pas vizës.
fn tokens(g: &str, split_hyphen: bool) -> Vec<String> {
    g.split([' ', ',', ';', ':'])
        .map(|t| t.trim_matches(['.', '\'', '’']).to_lowercase())
        .filter(|t| !t.is_empty())
        .map(|t| match split_hyphen {
            true => t.split('-').next().unwrap_or("").to_string(),
            false => t,
        })
        .filter(|t| !t.is_empty())
        .collect()
}

/// Gjatësia e pjesës së përbashkët në fillim të dy fjalëve (në shkronja).
fn common_prefix(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count()
}

/// A e përmend copa vetë fjalën (shembull përdorimi, jo shpjegim)? Vlen edhe trajta e lakuar ose
/// e ndryshuar në fund: «hangërshme» për «hangërshëm», «çerdha» për «çerdhë», «agullon» për
/// «agullue» (fillimi i përbashkët ≥ gjatësia − 2, të paktën 4 shkronja).
fn mentions(seg: &str, head: &str) -> bool {
    let h = head.to_lowercase();
    let hn = h.chars().count();
    let need = hn.saturating_sub(2).max(4);
    // Fjala e shkurtër me mbaresë («coku» për «cok»): nis me fjalën dhe është ≤ 3 shkronja më e gjatë.
    // Brenda një fraze (≥ 2 fjalë) vlen edhe forma e prishur nga OCR-ja («grafuiq» për «grahuq»,
    // «noreh» për «ngreh»): e njëjta shkronjë e parë, gjatësi ±1, largësia ≤ 1 (≤ 2 për fjalët me
    // ≥ 6 shkronja që nisin me të njëjtat dy shkronja). «guri» nuk është «gurrç», «gurë» s'është «gurinë».
    let toks: Vec<String> = tokens(seg, false).iter().flat_map(|t| t.split('-').map(|x| x.to_string()).collect::<Vec<_>>()).filter(|t| !t.is_empty()).collect();
    let fuzzy = |t: &str| {
        let tn = t.chars().count();
        let close_len = tn + 1 >= hn && tn <= hn + 1;
        let d = edit_distance(t, &h);
        let two = common_prefix(t, &h) >= 2;
        toks.len() >= 2 && hn >= 4 && common_prefix(t, &h) >= 1 && close_len && (d <= 1 || (d == 2 && hn >= 6 && two))
    };
    toks.iter().any(|t| {
        *t == h || common_prefix(t, &h) >= need || (hn >= 3 && t.starts_with(&h) && t.chars().count() <= hn + 3) || fuzzy(t)
    })
}

/// Shënjuesit e barazvlerësve të huaj: pjesa që nis me to nuk është shpjegim shqip.
const FOREIGN_MARKS: &[&str] = &["lat", "it", "ital", "gjerm", "germ", "frang", "fr", "tyrq", "turq", "sllav", "greq", "angl", "rum", "krh"];

/// Shënime gjuhësore për fjalën, jo shpjegim («me parashtesën ndë», «fjalë slave»).
const META_CLAUSES: &[&str] =
    &["me parashtes", "me prapashtes", "fjalë slave", "fjalë sllave", "fjalë turke", "fjalë e huej", "fjalë latine", "fjalë e vjetrueme"];

/// Pjesa nis me shënjues gjuhe të huaj («lat, abortivus», «it. rovelli») ose me shënim gjuhësor.
fn foreign(clause: &str) -> bool {
    let low = clause.trim().to_lowercase();
    META_CLAUSES.iter().any(|m| low.starts_with(m)) || foreign_mark(clause)
}

fn foreign_mark(clause: &str) -> bool {
    clause.split_whitespace().next().map(|t| FOREIGN_MARKS.contains(&t.trim_end_matches(['.', ',']).to_lowercase().as_str())).unwrap_or(false)
}

/// Zanore e dyfishtë («mee», «kacii»): shqipja e 1941 s'e ka, OCR-ja e prodhon.
fn doubled_vowel(t: &str) -> bool {
    ["aa", "ee", "ii", "oo", "uu", "yy", "ëë"].iter().any(|d| t.contains(d))
}

/// Mbaresat që Gazulli i shënon si copa më vete mes presjeve («bohçe, ja»).
const ENDING_CLAUSES: &[&str] = &["ja", "ni", "ri", "ia", "ut", "it", "në", "ët", "t", "u", "a", "i", "e", "së"];

/// Shpjegimi pa shembullin me vetë fjalën: pjesa e parë (sipas «:» ose «;») që nuk nis me të;
/// brenda saj, pjesët pas presjes ruhen vetëm deri te pjesa e parë që e përmend («qi hahet, bukë
/// e hangërshme» → «qi hahet»). Mbaresat si copa më vete hiqen.
fn without_example(g: &str, head: &str) -> Option<String> {
    g.split([':', ';'])
        .map(|seg| {
            seg.split(',')
                .map(|c| c.trim())
                .filter(|c| !c.is_empty() && !ENDING_CLAUSES.contains(&c.trim_end_matches('.')))
                .take_while(|c| !mentions(c, head) && !foreign(c))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .find(|s| !s.is_empty())
}

/// Etiketat e përdorimit ose të gramatikës që s'janë shpjegim («ndër shprehje», «thirrje»).
const LABELS: &[&str] = &[
    "në fjali", "në fraz", "grimcë", "thirrje", "ndër shprehje", "ndër përralla", "ndër fraza", "tue folë për", "trajtë",
    "trajta", "përdoret", "lakon", "kolektiv", "si emën", "shqiptue", "fjalë e vjetrueme", "fjalë e vjetër", "shms", "sme",
    "kënë", "emën vend", "emën i vend", "shumës", "njëjës", "e palakueshme", "i palakueshëm", "palak", "fj.", "fjal.",
    "fraz", "shih ",
];

/// Fjalë shumë të përgjithshme: e vetme, s'shpjegon asgjë (presja e preu: «njeri, bimë qi …»).
const GENERIC: &[&str] = &["njeri", "vend", "gjë", "gja", "send", "punë", "kjo", "ai", "ajo", "nji", "një"];

/// Folje ndihmëse: e vetme, është copë e prerë nga rreshti («qenka shtrëmbnue muri»).
const AUX: &[&str] = &["qenka", "ishte", "asht", "ësht", "është", "janë", "kanë", "kishte", "qe", "paska", "ka", "kë", "kam"];

/// Folje të lehta që pa kundrinor s'thonë gjë («me qitë», «me dhënë»).
const LIGHT: &[&str] = &["qitë", "bë", "ba", "dhënë", "dhanë", "vu", "vënë", "pasë", "marrë", "mbajtë", "lanë", "lënë", "çue"];

/// Largësia e redaktimit (Levenshtein) mes dy fjalëve, në shkronja.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != cb);
            cur.push(sub.min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Leksikët për kontrollin: `standard` = fjalët shqipe të burimeve të tjera dhe trajtat
/// morfologjike; `any` = edhe fjalët e vetë fjalorit të Gazullit.
pub struct Lexicon<'a> {
    pub standard: &'a dyn Fn(&str) -> bool,
    pub any: &'a dyn Fn(&str) -> bool,
}

/// Fjala e parë është `w`.
fn first_is(toks: &[String], w: &str) -> bool {
    toks.first().map(|t| t == w).unwrap_or(false)
}

/// Arsyeja e refuzimit, ose shpjegimi i pastër kur pranohet. `pos` = shënimi gramatikor i hyrjes
/// («n:m», «n:f», «a», «v», «r» ose bosh).
pub fn quality(lx: &Lexicon, head: &str, pos: &str, gloss: &str) -> Result<String, &'static str> {
    let trimmed = trim_edges(gloss.trim());
    let lower = trimmed.to_lowercase();
    let addendum = ["shto", "ndreq", "asht ", "particip", "shtypë"].iter().any(|p| lower.starts_with(p))
        || tokens(&lower, true).iter().any(|t| matches!(t.as_str(), "shto" | "ndreq" | "ndreqe" | "fshij"));
    // Referenca («sh. X» = shih X, «krh. X»): shpjegimi është vetëm pas «:», kur ka.
    let first_tok = lower.split_whitespace().next().unwrap_or("").trim_start_matches('(').to_string();
    let xref = matches!(first_tok.as_str(), "sh." | "sh" | "sh," | "shih" | "krh." | "krh" | "kr.");
    let body: Option<String> = match (xref, trimmed.split_once(':')) {
        (false, _) => Some(trimmed.clone()),
        (true, Some((_, after))) => Some(trim_edges(after.trim())),
        (true, None) => None,
    };
    let chosen = body.as_deref().and_then(|b| without_example(b, head));
    // Vizë në fund («treskë e ver-»): fjala u pre te fundi i rreshtit dhe vazhdimi humbi.
    let hyphen_cut = chosen
        .as_deref()
        .map(|c| c.trim_end_matches(['.', ',', ';', ':', ' ', '\'', '’']).ends_with('-'))
        .unwrap_or(false);
    let g = chosen.clone().unwrap_or_default();
    let g = trim_edges(&g).trim_end_matches(['.', ',', ';', ':', ' ', '\'', '’']).to_string();
    // «për plumb» = «plumb» (Gazulli e përdor «për» si «do të thotë»).
    let parts: Vec<&str> = g.split_whitespace().collect();
    let noun = matches!(pos, "n:m" | "n:f");
    let g = match (parts.as_slice(), noun) {
        (["për", x], true) => x.to_string(),
        _ => g.clone(),
    };
    let toks = tokens(&g, true);
    let raw = tokens(&g, false);
    let first = g.split_whitespace().next().unwrap_or("");
    let low_g = g.to_lowercase();
    let starts_grammar = matches!(first, "s." | "s.." | "m." | "f." | "sh." | "sh," | "fraz." | "fj." | "adj." | "adv." | "v.")
        || g.starts_with('-')
        // Mbaresat e lakimit si copa më vete («si, -t, gjeshku») janë shënim gramatikor.
        || g.split([' ', ',']).any(|t| t.starts_with('-'));
    let label = LABELS.iter().any(|p| low_g.starts_with(p)) || (toks.len() == 2 && first_is(&toks, "ndër"));
    // Duhet të paktën një e treta fjalë kuptimplote (jo vetëm nyje: «e, ja»), dhe shpjegimi s'mbaron
    // me një fjalë funksionale (OCR-ja e preu: «fillesa më e fjeshta e»).
    let content_n = toks.iter().filter(|t| t.chars().count() >= 3 && !SHORT_OK.contains(&t.as_str())).count();
    let last = toks.last().map(|t| t.as_str()).unwrap_or("");
    let dangling = DANGLING.contains(&last);
    let known_by = |f: &dyn Fn(&str) -> bool, t: &str| standard_candidates(t).iter().any(|c| f(c));
    let known = toks.iter().filter(|t| SHORT_OK.contains(&t.as_str()) || known_by(lx.any, t)).count();
    let short_junk = |t: &String| t.chars().count() <= 2 && !SHORT_OK.contains(&t.as_str());
    let junk = toks.iter().any(short_junk)
        || raw.iter().any(short_junk)
        || raw.iter().any(|t| OCR_TOKENS.contains(&t.as_str()) || doubled_vowel(t));
    // Shkronjë e madhe në mes (jo pas pike): emër latin bime, copë e huaj ose OCR; një fjalë e vetme
    // me shkronjë të madhe është emër vendi/fisi ose siglë.
    let words: Vec<&str> = g.split_whitespace().collect();
    let capital = words.iter().enumerate().skip(1).any(|(i, w)| {
        let core = w.trim_matches(|c: char| !c.is_alphabetic());
        core.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            && !words[i - 1].ends_with('.')
            && !PROPER.contains(&core)
    }) || (words.len() == 1 && first.chars().next().map(|c| c.is_uppercase()).unwrap_or(false));
    // Një fjalë e vetme: jo e përgjithshme, jo ndihmëse, jo copë ≤ 3 shkronja, dhe jo variant
    // drejtshkrimor i vetë fjalës («aht» → «aftë», «çafkë» → «çapkë») që s'e njeh fjalori standard.
    let one = toks.first().map(|t| t.as_str()).unwrap_or("");
    let weak_single = toks.len() == 1
        && (GENERIC.contains(&one)
            || AUX.contains(&one)
            || one.chars().count() <= 3
            || (edit_distance(one, &head.to_lowercase()) <= 2 && !known_by(lx.standard, one)));
    let light_verb = toks.len() == 2 && first_is(&toks, "me") && LIGHT.contains(&toks[1].as_str());
    // Emri, mbiemri ose ndajfolja nuk shpjegohen me një paskajore të zhveshur («arsim» ≠ «me dhënë»).
    // «me» si parafjalë («me fytyrë të prehët», «me nji kokërr») lejohet; paskajorja e zhveshur jo.
    let bare_infinitive = first_is(&toks, "me")
        && toks.len() <= 3
        && toks.get(1).map(|t| !SHORT_OK.contains(&t.as_str())).unwrap_or(false)
        && !head.ends_with("ue");
    let noun_as_verb = noun
        && ((first_is(&toks, "me") && toks.get(1).map(|t| !SHORT_OK.contains(&t.as_str())).unwrap_or(false))
            || first_is(&toks, "kam"));
    let pos_mismatch = (matches!(pos, "n:m" | "n:f" | "a" | "r") && bare_infinitive)
        || noun_as_verb
        // Emri nuk shpjegohet me një fjali të varur pa emër («kur nisë sythi», «qi bëjnë në dhë»).
        || (matches!(pos, "n:m" | "n:f") && (first_is(&toks, "kur") || first_is(&toks, "qi") || first_is(&toks, "që")));
    // Fjala e fundit e shkurtër duhet të jetë e njohur: ndryshe rreshti u pre në mes të fjalës
    // («fja», «njer»). Fjalët e gjata të panjohura janë zakonisht gege dhe i mat pragu 80%.
    let cut_word = !last.is_empty()
        && last.chars().count() <= 4
        && !SHORT_OK.contains(&last)
        && !known_by(lx.any, last)
        && !PROPER.iter().any(|p| p.to_lowercase() == last);
    // Apostrof i prishur nga OCR-ja («nr'u»): lejohen vetëm «m'», «s'», «n'», «t'», «k'», «ç'», «sh'».
    let bad_apostrophe = raw.iter().any(|t| {
        t.split_once(['\'', '’']).map(|(a, b)| !b.is_empty() && !matches!(a, "m" | "s" | "n" | "t" | "k" | "ç" | "sh" | "")).unwrap_or(false)
    });
    let shape_bad = toks.is_empty() || toks.len() > 18 || content_n == 0 || content_n * 3 < toks.len() || dangling;
    let checks: [(bool, &'static str); 12] = [
        (body.is_none(), "vetëm referencë (shih …), pa shpjegim"),
        (chosen.is_none(), "shembull me vetë fjalën, pa shpjegim"),
        (addendum, "shënim i shtojcës (shto/ndreq)"),
        (!ocr_clean(&g), "shenja të prishura nga OCR-ja"),
        (shape_bad, "gjatësi jashtë kufijve, pa fjalë kuptimplote ose e prerë"),
        (starts_grammar || label, "shënim gramatikor ose etiketë në vend të shpjegimit"),
        (junk || bad_apostrophe, "copa të shkurtra ose të prishura (OCR)"),
        (capital, "emër latin, emër vendi ose copë e huaj"),
        (cut_word || hyphen_cut, "fjala e fundit e prerë ose e panjohur"),
        (weak_single || light_verb, "fjalë e vetme pa shpjegim (e përgjithshme, ndihmëse ose variant)"),
        (pos_mismatch, "pjesë ligjërate që s'përputhet me shpjegimin"),
        (known * 10 < toks.len() * 8, "më pak se 80% fjalë të njohura"),
    ];
    match checks.iter().find(|(bad, _)| *bad) {
        Some((_, reason)) => Err(reason),
        None => Ok(drop_definite_endings(&g)),
    }
}

/// Neni i ngjitur te fjala e mbiemrit nga OCR-ja («fjalëmujshëmi-i» → «fjalëmujshëm»,
/// «mbërdhezëti» → «mbërdhezët») dhe «-ëa» («nocëa» → «nocë»).
fn clean_head(word: &str, grammar: &str) -> String {
    let glued_i = grammar == "a" && ["shëmi", "ëmi", "ëti", "emi", "uemi", "uni", "unri"].iter().any(|e| word.ends_with(e));
    let glued_a = word.ends_with("ëa") || word.ends_with("mj") || word.ends_with("nr");
    match glued_i || glued_a {
        true => word[..word.len() - 1].to_string(),
        false => word.to_string(),
    }
}

fn pos_attrs(grammar: &str) -> Vec<(String, String)> {
    match grammar {
        "n:m" => vec![("pos".into(), "n".into()), ("gender".into(), "m".into())],
        "n:f" => vec![("pos".into(), "n".into()), ("gender".into(), "f".into())],
        "a" => vec![("pos".into(), "a".into())],
        "v" => vec![("pos".into(), "v".into())],
        "r" => vec![("pos".into(), "r".into())],
        _ => vec![],
    }
}

/// Fjala e hyrjes është e qartë: vetëm shkronja, ≥ 3 gjithsej, çdo pjesë ≥ 2, jo fjalë funksionale.
fn word_ok(word: &str) -> bool {
    word.chars().all(|ch| ch.is_alphabetic() || ch == ' ')
        && word.chars().filter(|ch| ch.is_alphabetic()).count() >= 3
        && word.split_whitespace().all(|t| t.chars().count() >= 3)
        && !doubled_vowel(word)
        && !SHORT_OK.contains(&word)
        && !SHORT_OK.contains(&word.split_whitespace().next().unwrap_or(""))
}

pub fn import(g: &mut Graph, morph: &mut Morph, text: &str, rep: &mut ImportReport) -> usize {
    let mut rows = 0usize;
    let mut kept = 0usize;
    let mut new_words = 0usize;
    let mut reasons: BTreeMap<&'static str, usize> = BTreeMap::new();
    let lines: Vec<&str> = text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()).collect();
    // Leksiku para importit: fjalët shqipe që njohin burimet e tjera, trajtat morfologjike dhe
    // fjalët e vetë fjalorit (shpjegimet përdorin shpesh fjalë të tjera të Gazullit).
    let std_lex: HashSet<String> = g
        .nodes
        .iter()
        .filter_map(|n| match &n.kind {
            NodeKind::Word { lang: Lang::Sq, lemma } => Some(lemma.to_lowercase()),
            _ => None,
        })
        .collect();
    let gz_heads: HashSet<String> = lines
        .iter()
        .filter_map(|l| l.split('\t').next())
        .map(|w| w.trim().to_lowercase())
        .filter(|w| word_ok(w))
        .collect();
    let standard = |t: &str| std_lex.contains(t) || morph.knows_form(t);
    let any = |t: &str| standard(t) || gz_heads.contains(t);
    let lx = Lexicon { standard: &standard, any: &any };
    for l in lines {
        rows += 1;
        let c: Vec<&str> = l.split('\t').collect();
        let grammar = c.get(1).map(|s| s.trim()).unwrap_or("");
        let word = clean_head(&c.first().map(|s| s.trim().to_lowercase()).unwrap_or_default(), grammar);
        let gloss = c.get(2).map(|s| s.trim()).unwrap_or("");
        match (word_ok(&word), quality(&lx, &word, grammar, gloss)) {
            (false, _) => *reasons.entry("fjalë e paqartë").or_default() += 1,
            (true, Err(r)) => *reasons.entry(r).or_default() += 1,
            (true, Ok(clean)) => {
                let existed = g.get(&word_key(Lang::Sq, &word)).is_some();
                let w = g.ensure_word(Lang::Sq, &word);
                let mut attrs = pos_attrs(grammar);
                attrs.push(("region".into(), "veri".into()));
                attrs.push(("gloss".into(), clean));
                attrs.push(("src".into(), "gazulli1941".into()));
                g.add_edge(Edge {
                    from: w,
                    to: w,
                    rel: Rel::Sem(REL.into()),
                    source: Source::Gazulli1941,
                    lang: Some(Lang::Sq),
                    method: Method::Published,
                    attrs,
                });
                kept += 1;
                match existed {
                    true => {}
                    false => new_words += 1,
                }
            }
        }
    }
    rep.note(format!(
        "gazulli1941: {rows} hyrje në TSV; {kept} shpjegime krahinore të pranuara ({new_words} fjalë të reja në graf); të refuzuara sipas arsyes {reasons:?}"
    ));
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lexicon(t: &str) -> bool {
        [
            "shpend", "vogël", "pendla", "verdhëm", "i", "fortë", "guri", "britmë", "keqe", "zë", "naltë", "lartë", "muzg",
            "përkulur", "jashtëm", "terratis", "bimë", "liqen", "hahet", "avull", "pip", "lisi", "ri", "plumb", "akull", "hej",
            "ngrehen", "grue", "madhe",
        ]
        .contains(&t)
    }

    fn q(head: &str, gloss: &str) -> Result<String, &'static str> {
        quality(&Lexicon { standard: &lexicon, any: &lexicon }, head, "", gloss)
    }

    #[test]
    fn keeps_clean_glosses_and_rejects_ocr_noise() {
        assert_eq!(q("afçë", "shpend i vogël me pendla").unwrap(), "shpend i vogël me pendla");
        assert!(q("afçë", "i fortë si guri").is_ok());
        assert_eq!(q("mugëz", "muzg-u").unwrap(), "muzg", "trajtë e lidhur me fjalë standarde, pa mbaresë");
        assert!(q("abe", "jë 66 si abe : j£ bë si hije").is_err());
        assert!(q("afçë", "s. £. -e, t: calinë").is_err());
        assert!(q("afçë", "-a, t: jelëk").is_err());
        assert!(q("afçë", "me nji pë të gja të të njyem me bë viza ië drejta në dri").is_err());
        assert!(q("afçë", "e, ja").is_err(), "vetëm nyje");
        assert!(q("afçë", "si, -t, muzg").is_err(), "mbaresë si copë");
        assert!(q("afçë", "shpend i vogël me").is_err(), "i prerë");
        assert!(q("afçë", "shpend i vogël si").is_err(), "i prerë te «si»");
        assert_eq!(q("afçë", "ady. shpend i vogël").unwrap(), "shpend i vogël");
        assert_eq!(q("afçë", "shpend i vogël. I").unwrap(), "shpend i vogël");
        assert_eq!(q("afçë", "shpend i vogël. muzg-a").unwrap(), "shpend i vogël");
    }

    #[test]
    fn examples_addenda_latin_names_and_known_ocr_garbage_are_not_glosses() {
        // Shembulli me vetë fjalën: merret pjesa që e shpjegon.
        assert_eq!(q("agullue", "më janë agullue sytë: terratisë").unwrap(), "terratisë");
        assert!(q("agullue", "më janë agullue sytë").is_err());
        assert_eq!(q("hangërshëm", "qi hahet, bukë e hangërshme").unwrap(), "qi hahet");
        assert!(q("çerdhë", "çerdhe, ja, ndërtoi shpin çerdha çerdha").is_err());
        assert!(q("cok", "coku i gishtit").is_err(), "shembull me temën e fjalës");
        assert_eq!(q("futë", "bimë, ja").unwrap(), "bimë");
        assert!(q("kështarpë", "bimë, shto").is_err());
        assert!(!mentions("njeri qi e kë menden akull", "akulltar"), "fjala bazë nuk është përmendje");
        assert!(q("afçë", "shto: shpend i vogël").is_err());
        assert_eq!(q("boshtërr", "nji bimë Ligustrum vulgare"), Err("emër latin, emër vendi ose copë e huaj"));
        assert!(q("lëqëth", "liqen i vogël i Shkodrës").is_ok());
        assert!(q("afçë", "shpend dsht i vogël").is_err());
        assert!(q("afçë", "e palakueshme").is_err());
        assert_eq!(q("shtijak", "bimë e vogël, lat, abortivus").unwrap(), "bimë e vogël");
        assert!(q("ndërmend", "mee").is_err());
        assert!(q("mëdhaj", "shumës").is_err());
        assert!(!word_ok("tashëmii"));
        assert!(!word_ok("kësh ns"));
        // Shembulli me formën e prishur të fjalës bie; pjesa pas «:» mbetet.
        assert_eq!(q("katanë", "grue katënë: grue e madhe").unwrap(), "grue e madhe");
        // Emri nuk shpjegohet me fjali të varur; fjala e prerë në fund bie.
        assert!(quality(&Lexicon { standard: &lexicon, any: &lexicon }, "krandë", "n:f", "kur nisë sythi").is_err());
        assert!(q("sufrue", "shpend i vogël fja").is_err());
        assert!(q("motsue", "shpend nr'u vogël").is_err());
        assert_eq!(trim_edges("'v.'as. shpend i vogël"), "shpend i vogël");
    }

    #[test]
    fn cross_references_labels_variants_and_weak_single_words_are_not_glosses() {
        // «sh. X» = shih X: pa shpjegim; me «:» merret shpjegimi pas saj.
        assert_eq!(q("domriz", "sh. domlic"), Err("vetëm referencë (shih …), pa shpjegim"));
        assert_eq!(q("filashnjok", "sh. bilashnjok: pip lisi të ri").unwrap(), "pip lisi të ri");
        // Etiketat e përdorimit.
        assert!(q("hejdi", "ndër shprehje").is_err());
        assert!(q("flakakuq", "ndër përralla").is_err());
        assert!(q("qëllol", "grimcë thirrëse").is_err());
        // Variant drejtshkrimor që s'e njeh fjalori standard; forma standarde pranohet.
        assert!(q("çafkë", "çapkë").is_err());
        assert_eq!(q("abull", "avull").unwrap(), "avull");
        // Fjalë e vetme e përgjithshme, ndihmëse ose folje e lehtë.
        assert!(q("dimnak", "njeri").is_err());
        assert!(q("gjëmue", "qenka").is_err());
        assert!(q("capllue", "me qitë").is_err());
        // «për plumb» = «plumb».
        assert_eq!(quality(&Lexicon { standard: &lexicon, any: &lexicon }, "çum", "n:m", "për plumb").unwrap(), "plumb");
        // Emri nuk shpjegohet me paskajore të zhveshur; «me» si parafjalë lejohet.
        assert!(quality(&Lexicon { standard: &lexicon, any: &lexicon }, "prehaluc", "a", "me grue të madhe").is_ok());
        assert!(q("pështirë", "i fortë").is_ok(), "fjalë e ngjashme me shkronjë tjetër nuk është përmendje");
        assert!(quality(&Lexicon { standard: &lexicon, any: &lexicon }, "arsim", "n:m", "me dhënë").is_err());
        // Pjesa e dytë e fjalës së përbërë mbetet.
        assert_eq!(drop_definite_endings("i pa-matun"), "i pa-matun");
        assert_eq!(drop_definite_endings("muzg-u, avull-i"), "muzg, avull");
        assert_eq!(edit_distance("aht", "aftë"), 2);
        assert_eq!(clean_head("fjalëmujshëmi", "a"), "fjalëmujshëm");
        assert_eq!(clean_head("nocëa", "n:f"), "nocë");
        assert_eq!(clean_head("kakërdhoçë", "n:f"), "kakërdhoçë");
        assert_eq!(clean_head("vrashëmj", "a"), "vrashëm");
        assert!(q("mbajtun", "njeri i mbaj-'").is_err());
        assert_eq!(trim_edges("s. m.j, shpend i vogël"), "shpend i vogël");
        assert_eq!(q("ndojnik", "bimë, me parashtesën ndë").unwrap(), "bimë");
        // Vizë në fund, fjalë e përbërë me vetë fjalën, «për X» te ndajfolja, emër me «me + folje».
        assert!(q("çingël", "treskë e ver-").is_err());
        assert!(q("grap", "njeri bark-grap").is_err());
        assert!(quality(&Lexicon { standard: &lexicon, any: &lexicon }, "flauri", "r", "për plumb").unwrap() == "për plumb");
        assert!(quality(&Lexicon { standard: &lexicon, any: &lexicon }, "rrmone", "n:f", "me çilë hendek").is_err());
    }

    #[test]
    fn gheg_forms_are_checked_against_standard_forms() {
        assert!(standard_candidates("përkulun").contains(&"përkulur".to_string()));
        assert!(standard_candidates("punue").contains(&"punuar".to_string()));
        assert!(standard_candidates("nji").contains(&"një".to_string()));
        assert!(standard_candidates("liqenit").contains(&"liqen".to_string()));
    }

    #[test]
    fn heads_need_letters_and_no_single_letter_parts() {
        assert!(word_ok("kakërdhoçë"));
        assert!(!word_ok("a ngulë"));
        assert!(!word_ok("me"));
        assert!(!word_ok("ab"));
    }
}
