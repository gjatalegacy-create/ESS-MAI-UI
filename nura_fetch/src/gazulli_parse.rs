//! Ndarja e tekstit OCR të fjalorit të Gazullit (1941) në hyrje: fjala, shënimi gramatikor,
//! shpjegimi shqip. Rregulla të deklaruara dhe deterministike:
//!
//! 1. Trupi i fjalorit: rreshtat pas rreshtit «FJALUER» dhe para rreshtit «FUND» (hiqen parathënia,
//!    shkurtesat dhe studimi i parashtesave; shtojca «Fjalë për t'u plotsue» mbetet, por shënimet
//!    e saj «shto/ndreq» i hedh importuesi).
//! 2. Hiqen numrat e faqeve, shkronjat e vetme të seksioneve dhe rreshtat pa shkronja.
//! 3. Paragrafët ndahen me rresht bosh; fjala e ndarë me vizë në fund të rreshtit bashkohet. Kur një
//!    paragraf mbaron me vizë dhe i pasuesi nuk është hyrje, bashkohen (ndarje faqeje).
//! 4. FORMA E HYRJES: fjalë me shkronja të vogla shqipe; pas saj (dhe pas mbaresave të shkurtra si
//!    «-a, t») vjen një kllapë me sigla krahine ose një shkurtim gramatikor («s. m.», «adj.», «v.» …).
//!    Ndryshe paragrafi është vazhdim i hyrjes së mëparshme dhe nuk merret si hyrje.
//! 5. SEKSIONI ALFABETIK: shkronja e parë e fjalës (me dyshkronjëshat dh, gj, ll, nj, rr, sh, th, xh,
//!    zh) duhet të jetë ajo e shumicës së 3 hyrjeve para ose 3 hyrjeve pas (një copë e humbur nga
//!    faqja tjetër bie jashtë seksionit).
//! 6. Fjala: pjesa para «(», «:» ose «,»; pas vizës vjen një mbaresë e njohur («abe-ja» → «abe»).
//!    Fjala e përbërë me vizë («krymb-bore») ose me emër para saj në kllapë («(gjuhë) qëni») nuk
//!    shkurtohet në pjesën e parë: hyrja hidhet.
//! 7. Shpjegimi: pas siglave dhe shkurtimit gramatikor; pritet te kuptimi i dytë («2.»), te
//!    barazvlerësi i parë i huaj («gjerm.», «frang.», «lat.», «it.» …) dhe te krahasimi («krh.»).

/// Një hyrje e fjalorit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub word: String,
    /// «n:m», «n:f», «a», «v», «r» ose bosh.
    pub grammar: String,
    pub gloss: String,
}

fn albanian_letter(c: char) -> bool {
    c.is_ascii_lowercase() || matches!(c, 'ç' | 'ë')
}

/// Rresht që s'është tekst: numër faqeje («— 12 —»), shkronjë seksioni, pa shkronja.
fn noise_line(l: &str) -> bool {
    let t = l.trim();
    let letters = t.chars().filter(|c| c.is_alphabetic()).count();
    letters == 0 || t.chars().count() <= 3
}

/// Paragrafët e trupit të fjalorit, me rreshtat e bashkuar.
pub fn paragraphs(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|l| l.trim() == "FJALUER").map(|i| i + 1).unwrap_or(0);
    let end = lines.iter().rposition(|l| l.trim() == "FUND").unwrap_or(lines.len());
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    for l in lines[start..end.max(start)].iter() {
        let t = l.trim();
        match (t.is_empty(), noise_line(t)) {
            (true, _) => {
                match cur.trim().is_empty() {
                    true => {}
                    false => out.push(cur.trim().to_string()),
                }
                cur = String::new();
            }
            (false, true) => {}
            (false, false) => {
                // Fjala e ndarë me vizë në fund të rreshtit: «fi-» + «nesse» → «finesse».
                match (cur.ends_with('-'), t.chars().next().map(albanian_letter).unwrap_or(false)) {
                    (true, true) => {
                        cur.pop();
                        cur.push_str(t);
                    }
                    _ => {
                        match cur.is_empty() {
                            true => {}
                            false => cur.push(' '),
                        }
                        cur.push_str(t);
                    }
                }
            }
        }
    }
    match cur.trim().is_empty() {
        true => {}
        false => out.push(cur.trim().to_string()),
    }
    out
}

/// Mbaresat e shquara dhe të lakimit që Gazulli shënon pas vizës.
const ENDINGS: &[&str] = &[
    "a", "i", "ja", "e", "u", "t", "ni", "ore", "ë", "ori", "it", "oni", "olli", "ia", "elli", "ëlli", "ii", "uri", "ri",
    "oili", "j", "ët", "ta", "na", "ri", "ja", "ëja", "ëri", "",
];

/// Emrat që Gazulli vë në kllapë para fjalës për një term të përbërë («(gjuhë) qëni» = gjuhë qëni).
const COMPOUND_HEADS: &[&str] = &[
    "gjuhë", "gojë", "bar", "rrush", "fik", "gur", "zog", "pulë", "lundër", "peshku", "leshi", "lule", "hardhi",
    "sëpatë", "vesh", "bukë", "miell", "lejthi", "kënete",
];

/// Fjala, a është e përbërë, dhe pjesa pas saj, kur paragrafi ka formën e një hyrjeje.
fn head(p: &str) -> Option<(String, String)> {
    let t = p.trim_start_matches([',', '.', '\'', '’', ' ', '‘']);
    let cut = t.find(['(', ':', ',']).unwrap_or(t.len());
    let raw = t[..cut].trim();
    let rest = t[cut..].to_string();
    let words: Vec<&str> = raw.split_whitespace().collect();
    let first_ok = words.first().map(|w| w.chars().next().map(albanian_letter).unwrap_or(false)).unwrap_or(false);
    let clean = words.iter().all(|w| w.chars().all(|c| albanian_letter(c) || c == '-') && w.chars().count() >= 2)
        && words.len() <= 2;
    // Pas vizës duhet një mbaresë e njohur; ndryshe fjala është e përbërë («krymb-bore»).
    let compound = words.iter().any(|w| match w.split_once('-') {
        Some((_, end)) => !ENDINGS.contains(&end.split('-').next().unwrap_or("")),
        None => false,
    });
    let lemma = words.first().map(|w| w.split('-').next().unwrap_or("").to_string()).unwrap_or_default();
    let lemma = match words.len() {
        2 => format!("{lemma} {}", words[1].split('-').next().unwrap_or("")),
        _ => lemma,
    };
    // Forma: pas mbaresave të shkurtra vjen një kllapë ose një shkurtim gramatikor.
    let after = skip_endings(&rest);
    let shaped = after.starts_with('(') || grammar_mark(&after);
    // Emër i përbërë në kllapën e parë («(gjuhë M.M.)»).
    let paren_head = after
        .strip_prefix('(')
        .and_then(|x| x.split([' ', '.', ',', ')']).next())
        .map(|x| COMPOUND_HEADS.contains(&x))
        .unwrap_or(false);
    match (first_ok && clean && shaped && !compound && !paren_head, lemma.chars().filter(|c| c.is_alphabetic()).count() >= 2) {
        (true, true) => Some((lemma, rest)),
        _ => None,
    }
}

/// Kapërcen «:», «,» dhe copat e shkurtra të mbaresave («e, t», «-ja») në fillim.
fn skip_endings(s: &str) -> String {
    let mut t = s.trim_start_matches([':', ',', ' ', ';']).to_string();
    loop {
        let tok: String = t.chars().take_while(|c| !matches!(c, ' ' | ',' | ':' | ';' | '(')).collect();
        let short = !tok.is_empty() && (tok.starts_with('-') || (tok.chars().count() <= 3 && tok.chars().all(|c| c.is_alphabetic())));
        // Shkurtimet gramatikore («s.», «v.») nuk janë mbaresa.
        match (short, grammar_mark(&t)) {
            (true, false) => t = t[tok.len()..].trim_start_matches([':', ',', ' ', ';']).to_string(),
            _ => return t,
        }
    }
}

/// Teksti nis me një shkurtim gramatikor (edhe në trajtat e dëmtuara nga OCR-ja).
fn grammar_mark(s: &str) -> bool {
    let low = s.to_lowercase();
    let compact: String = low.chars().take(10).filter(|c| !c.is_whitespace()).collect();
    [
        "adj", "adi", "adv", "ady", "ad.", "akt", "s.", "st.", "si.", "s-", "sm.", "ns.", "v.", "v-", "vas.", "vrefl", "prep",
        "përëm", "grimc", "fj.", "lakon", "pas.",
    ]
    .iter()
    .any(|m| compact.starts_with(m))
        || low.starts_with("s ")
        || low.starts_with("v ")
}

/// Heq siglat e krahinave në kllapa nga fillimi dhe kthen pjesën tjetër.
fn drop_leading_parens(s: &str) -> String {
    let mut t = s.trim_start_matches([':', ',', ' ', ';']).to_string();
    loop {
        match (t.starts_with('('), t.find(')')) {
            (true, Some(i)) => t = t[i + 1..].trim_start_matches([':', ',', ' ', ';']).to_string(),
            _ => return t,
        }
    }
}

/// Shkurtimi gramatikor në fillim («s. m.», «s.f.», «adj.», «adv.», «v. a.») dhe pjesa pas tij.
fn grammar(s: &str) -> (String, String) {
    let t = s.trim_start();
    let compact: String = t.to_lowercase().chars().take(14).filter(|c| !c.is_whitespace()).collect();
    let starts = |xs: &[&str]| xs.iter().any(|x| compact.starts_with(x));
    let g = match (
        starts(&["adj", "adi"]),
        starts(&["adv", "ady"]),
        starts(&["s..m", "s.m", "sm.", "s-m", "ns.m"]),
        // OCR-ja e lexon «f» si «£», «Ë», «t» ose «i».
        starts(&["s..f", "s.f", "s.£", "s.ë", "st.", "s.t.", "s.i.", "si."]),
        starts(&["v.", "vas.", "vrefl", "s.akt", "pas."]),
    ) {
        (true, _, _, _, _) => "a",
        (_, true, _, _, _) => "r",
        (_, _, true, _, _) => "n:m",
        (_, _, _, true, _) => "n:f",
        (_, _, _, _, true) => "v",
        _ => "",
    };
    match g.is_empty() {
        true => (String::new(), t.to_string()),
        false => {
            // Shkurtimi: vetëm fjalët e shkurtimeve të njohura («s.», «m.», «adj.», «v. akt.»), jo
            // shpjegimi që vjen pas tyre («s. m. muzg-u.» → «muzg-u.»).
            // «e» lidh dy shkurtime («v. as. e akt.»).
            let mut rest = t;
            loop {
                let mut it = rest.split_whitespace();
                let tok = it.next().unwrap_or("");
                let next = it.next().unwrap_or("");
                match (abbr_token(tok) || (tok == "e" && abbr_token(next)), tok.is_empty()) {
                    (true, false) => rest = rest.trim_start()[tok.len()..].trim_start(),
                    _ => break,
                }
            }
            (g.to_string(), sense_one(&drop_endings(rest.trim_start_matches([':', ',', ' ', ';']))))
        }
    }
}

/// Copat e shkurtimeve gramatikore të Gazullit (edhe në leximet e OCR-së: «£», «ë», «i» për «f»).
const ABBR: &[&str] = &[
    "s", "m", "f", "£", "ë", "t", "i", "adj", "adi", "adv", "ady", "ad", "v", "a", "akt", "pës", "pas", "refl", "vrefl", "as",
    "n", "ns", "sm", "st", "si", "sf", "vas", "prep", "përëm", "përem", "grimc", "lakon", "jokal", "kal", "shms",
];

/// Fjalë shkurtimi: copat e saj mes pikave janë shkurtime të njohura dhe mbaron me pikë («s..m.»,
/// «adj.», «v.»); «akt», «pës», «refl» lejohen edhe pa pikë.
fn abbr_token(tok: &str) -> bool {
    let t = tok.trim_end_matches([',', ':', ';']);
    let pieces: Vec<String> = t.split('.').filter(|p| !p.is_empty()).map(|p| p.to_lowercase()).collect();
    let known = !pieces.is_empty() && pieces.iter().all(|p| ABBR.contains(&p.as_str()));
    known && (t.contains('.') || matches!(t, "akt" | "pës" | "refl" | "jokal" | "kal"))
}

/// Kur kuptimi i parë shënohet me «1.» pak pas fillimit (pas mbaresave të prishura nga OCR-ja:
/// «efaj t. 1. akull …»), shpjegimi nis aty.
fn sense_one(s: &str) -> String {
    match s.find("1.").filter(|&i| i <= 14) {
        Some(i) => s[i + 2..].trim_start().to_string(),
        None => s.to_string(),
    }
}

/// Heq mbaresat e lakimit pas shkurtimit gramatikor («-a, t:», «-e, t:», «fa, £:», «m-a, t:»).
fn drop_endings(s: &str) -> String {
    let head: String = s.chars().take(14).collect();
    let short = head.find(':').map(|i| &head[..i]).unwrap_or("");
    let looks_like_endings = !short.is_empty()
        && short
            .split([',', ' '])
            .filter(|x| !x.is_empty())
            .all(|x| x.starts_with('-') || x.chars().count() <= 2 || (x.chars().count() <= 5 && x.contains('-')));
    match looks_like_endings {
        true => s[short.len()..].trim_start_matches([':', ',', ' ', ';']).to_string(),
        false => s.to_string(),
    }
}

/// Shenjat që hapin barazvlerësit e huaj, krahasimet ose një kuptim tjetër: shpjegimi pritet aty.
const CUTS: &[&str] = &[
    " gjerm.", " germ.", " frang.", " trang.", " fr.", " it.", " ital.", " lat.", " rum.", " angl.", " greq.",
    " sllav.", " tyrq.", " turq.", "(krh", "(kr.", "(krah", " krh.", " 2.", " 2 .", " shms",
    // Kllapat në mes të shpjegimit mbajnë sigla krahinash ose nisin një hyrje të bashkuar nga OCR-ja.
    "(",
];

fn cut_gloss(s: &str) -> String {
    let t = format!(" {}", s.trim());
    let lower = t.to_lowercase();
    let end = CUTS.iter().filter_map(|c| lower.find(c)).min().unwrap_or(t.len());
    let g = t[..end].trim();
    let g = g.strip_prefix("1.").unwrap_or(g).trim();
    g.trim_end_matches([':', ',', ';', ' ', '—', '-']).to_string()
}

/// Variant i fjalës me sigla krahine para shpjegimit («aftë (M.M.) 1. e fshëme» → «1. e fshëme»;
/// «dërgue (M.M) viçi ndërgon» → «viçi ndërgon»): fjala e vetme para kllapës nuk është shpjegim.
fn skip_variant(s: &str) -> String {
    let t = s.trim_start();
    let parts = t
        .split_once('(')
        .and_then(|(w, rest)| rest.split_once(')').map(|(inside, after)| (w.trim(), inside, after)))
        .filter(|(_, inside, _)| sigla(inside))
        .map(|(w, _, after)| (w, after));
    match parts {
        Some((w, after)) => match (!w.is_empty() && !w.contains(' '), after.trim_start_matches([':', ',', ' ', ';', '.'])) {
            (true, a) => match a.is_empty() {
                true => t.to_string(),
                false => a.to_string(),
            },
            (false, _) => t.to_string(),
        },
        None => t.to_string(),
    }
}

/// Siglat e krahinave/burimeve në kllapë: nisin me shkronjë të madhe, vetëm shkronja, pika,
/// presje dhe hapësira («M.M.», «SP. PK. MR», «Shest»); «dheu» nuk është siglë.
fn sigla(inside: &str) -> bool {
    let t = inside.trim();
    t.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        && t.chars().count() <= 24
        && t.chars().all(|c| c.is_alphabetic() || matches!(c, '.' | ',' | ' '))
}

/// Pas një kllape me sigla (brenda 60 shenjave të para) vjen shkurtimi gramatikor: pjesa që nis aty.
fn variant_then_grammar(body: &str) -> Option<String> {
    body.char_indices()
        .filter(|(i, c)| *c == ')' && *i < 60)
        .map(|(i, _)| body[i + 1..].trim_start_matches([':', ',', ' ', ';']))
        .find(|r| grammar_mark(r))
        .map(|r| r.to_string())
}

/// Shkronja e parë e fjalës për seksionin alfabetik (me dyshkronjëshat e shqipes).
fn initial(w: &str) -> String {
    let two: String = w.chars().take(2).collect();
    match two.as_str() {
        "dh" | "gj" | "ll" | "nj" | "rr" | "sh" | "th" | "xh" | "zh" => two,
        _ => w.chars().take(1).collect(),
    }
}

/// Shkronja më e shpeshtë në një grup fjalësh (barazimi: e para në renditje).
fn majority(ws: &[&str]) -> Option<String> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for w in ws {
        let i = initial(w);
        match counts.iter_mut().find(|(x, _)| *x == i) {
            Some(c) => c.1 += 1,
            None => counts.push((i, 1)),
        }
    }
    counts.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))).map(|x| x.0)
}

/// Hyrjet e fjalorit, sipas rregullave të modulit.
pub fn entries(text: &str) -> Vec<Entry> {
    // Paragrafët me formë hyrjeje; një paragraf që mbaron me vizë bashkohet me vazhdimin e tij.
    let paras = paragraphs(text);
    let mut cands: Vec<(String, String)> = Vec::new();
    let mut open_hyphen = false;
    for p in paras {
        match (head(&p), open_hyphen) {
            (Some((word, rest)), _) => {
                open_hyphen = p.ends_with('-');
                cands.push((word, rest));
            }
            (None, true) => {
                open_hyphen = p.ends_with('-');
                match cands.last_mut() {
                    Some(last) => {
                        last.1.pop();
                        last.1.push_str(p.trim());
                    }
                    None => {}
                }
            }
            // Vazhdim pa vizë: shpjegimi pritet gjithsesi te kuptimi i parë, prandaj s'merret.
            (None, false) => open_hyphen = false,
        }
    }
    // Seksioni alfabetik: fjala duhet të ndajë shkronjën e parë me fqinjët e saj.
    let words: Vec<String> = cands.iter().map(|(w, _)| w.clone()).collect();
    let mut out: Vec<Entry> = Vec::new();
    for (i, (word, rest)) in cands.iter().enumerate() {
        let before: Vec<&str> = words[i.saturating_sub(3)..i].iter().map(|s| s.as_str()).collect();
        let after: Vec<&str> = words[(i + 1).min(words.len())..(i + 4).min(words.len())].iter().map(|s| s.as_str()).collect();
        let me = initial(word);
        let in_section = majority(&before).map(|m| m == me).unwrap_or(true) || majority(&after).map(|m| m == me).unwrap_or(true);
        match in_section {
            true => {
                let body = drop_leading_parens(&skip_endings(rest));
                let (g, after) = grammar(&body);
                // Variant me sigla para shënimit gramatikor («gjaja (M.M) djajë-a (PK) s.f. 1. gjojë»):
                // shpjegimi nis pas shënimit, jo te varianti.
                let (g, after) = match (g.is_empty(), variant_then_grammar(&body)) {
                    (true, Some(b)) => grammar(&b),
                    _ => (g, after),
                };
                let after = sense_one(&skip_variant(&drop_leading_parens(&after)));
                out.push(Entry { word: word.clone(), grammar: g, gloss: cut_gloss(&after) });
            }
            false => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "parathënie\nFJALUER \n\n\n'abe-ja:: (Krz, MR) s..m. 1. jë 66 si abe : j£ bë si hije: \ngjerm. Schattierung: trang. nuance.\n\nabull-i: (KL.MM.) s.m. avull-i (krh. rumenishten : adur.) \n\nafçë-a (Rr. M.M) s. f.: shpend i vogël me pendla \ntë verdhëmta : frang. verdier: it. verdone. \n\nE \n\nagullue (PK) v. akt. më janë agullue sjt: terratisë. \n\n— 12 —\n\nakulltar-e (SP) adj. njeri qi e kë menden akull, qi \ni peshon fjalët. \n\nFUND \nNDREQIME GABIMESH\n";

    #[test]
    fn splits_real_entries_and_cuts_foreign_equivalents() {
        let e = entries(SAMPLE);
        let words: Vec<&str> = e.iter().map(|x| x.word.as_str()).collect();
        assert_eq!(words, vec!["abe", "abull", "afçë", "agullue", "akulltar"]);
        let afce = e.iter().find(|x| x.word == "afçë").unwrap();
        assert_eq!(afce.grammar, "n:f");
        assert_eq!(afce.gloss, "shpend i vogël me pendla të verdhëmta");
        let ak = e.iter().find(|x| x.word == "akulltar").unwrap();
        assert_eq!(ak.grammar, "a");
        assert_eq!(ak.gloss, "njeri qi e kë menden akull, qi i peshon fjalët.");
        assert_eq!(e.iter().find(|x| x.word == "abull").unwrap().gloss, "avull-i");
        assert_eq!(e.iter().find(|x| x.word == "agullue").unwrap().grammar, "v");
        assert_eq!(drop_endings("-a, t: ballanxhë, depozitë"), "ballanxhë, depozitë");
        assert_eq!(drop_endings("m-a, t: jelëk-u"), "jelëk-u");
        assert_eq!(drop_endings("shpend i vogël"), "shpend i vogël");
        // Shkurtimi nuk gëlltit shpjegimin.
        assert_eq!(grammar("s. m. muzg-u."), ("n:m".to_string(), "muzg-u.".to_string()));
        assert_eq!(grammar("adj. i kuq si gjaku"), ("a".to_string(), "i kuq si gjaku".to_string()));
        assert_eq!(grammar("v. akt. me mungue"), ("v".to_string(), "me mungue".to_string()));
        assert_eq!(grammar("v. as. e akt. kujtue"), ("v".to_string(), "kujtue".to_string()));
        assert_eq!(grammar("s.m “e, £: kërkim, kërkesë."), ("n:m".to_string(), "kërkim, kërkesë.".to_string()));
        assert_eq!(grammar("s. m. shms. të dy të martuemit"), ("n:m".to_string(), "të dy të martuemit".to_string()));
        assert_eq!(grammar("s. m. efaj t. 1. akull si hej"), ("n:m".to_string(), "akull si hej".to_string()));
        assert_eq!(variant_then_grammar("djajë-a (PK) s.f. 1. gjojë").as_deref(), Some("s.f. 1. gjojë"));
        assert_eq!(sense_one(&skip_variant("aftë (M.M.) 1. e fshëme, fshatë")), "e fshëme, fshatë");
        assert_eq!(skip_variant("dërgue (M.M) viçi ndërgon"), "viçi ndërgon");
        assert_eq!(skip_variant("ënë (dheu) me bark të madhe"), "ënë (dheu) me bark të madhe");
        assert_eq!(skip_variant("shpend i vogël (M.M) me pendla"), "shpend i vogël (M.M) me pendla");
        assert_eq!(sense_one("çapkë-a. 1. çafkëbardhë-a"), "çafkëbardhë-a");
    }

    #[test]
    fn page_numbers_and_section_letters_are_not_text() {
        assert!(noise_line("— 12 —"));
        assert!(noise_line("E"));
        assert!(!noise_line("ag-u (SP. PK.) s. m."));
    }

    #[test]
    fn continuations_compounds_and_out_of_section_words_are_not_entries() {
        // Vazhdim pa formë hyrjeje (pa kllapë/shkurtim pas fjalës).
        assert!(head("rampino, drti a hekur i krrutzuem si grep.").is_none());
        // Fjalë e përbërë me vizë dhe me emër në kllapë.
        assert!(head("krymb-bore (M.M) s. m. nji farë krymbi").is_none());
        assert!(head("qëni (gjuhë M.M.) s. i. Cynoglossum").is_none());
        // Hyrje e rregullt me mbaresa para siglave.
        assert!(head("aht-i,e, t (PK) aftë (M.M.) 1. e fshëme").is_some());
        // Fjala jashtë seksionit alfabetik bie.
        let text = "FJALUER\n\nbabë-a (SP) s. f. nënë\n\nbaba-i (SP) s. m. at\n\nbali-a (SP) s. f. ballë\n\nzogël (SP) s. f. zog i vogël\n\nbarc-i (SP) s. m. thes\n\nbard-i (SP) s. m. sqep\n\nbark-u (SP) s. m. stomak\n\nFUND\n";
        let ws: Vec<String> = entries(text).into_iter().map(|e| e.word).collect();
        assert!(!ws.contains(&"zogël".to_string()), "{ws:?}");
        assert!(ws.contains(&"barc".to_string()));
    }

    #[test]
    fn a_hyphen_at_a_page_break_joins_the_continuation() {
        let text = "FJALUER\n\ndumllue (SP) v. akt. me ushtrue, me gjimue, dum-\n\nllue fort\n\nFUND\n";
        let e = entries(text);
        assert_eq!(e[0].gloss, "me ushtrue, me gjimue, dumllue fort");
    }
}
