//! Grafi i fjalëve dhe kuptimeve.
//!
//! Parimet (nga mandati dhe nga ligji i ESS-MAI):
//! - Fjala (`Word`) dhe kuptimi (`Sense`) janë nyje të ndryshme me identitete të
//!   dallueshme; një fjalë mund të ketë shumë kuptime dhe asnjë kuptim nuk
//!   bashkohet me një tjetër vetëm sepse ndajnë të njëjtën fjalë.
//! - Çdo skaj mbart burimin, gjuhën dhe metodën me të cilën u krijua.
//!   Lidhjet ndërmjet burimeve kanë metodë të deklaruar (`Method`).
//! - Skaji (marrëdhënia semantike/gjuhësore) është i ndryshëm nga vija që
//!   vizatohet në ekran; paraqitja vizuale ndërtohet më vonë nga `layout`.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Lang {
    Sq,
    En,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::Sq => "sq",
            Lang::En => "en",
        }
    }
    pub fn parse(s: &str) -> Option<Lang> {
        match s {
            "sq" => Some(Lang::Sq),
            "en" => Some(Lang::En),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Source {
    /// Princeton WordNet 3.0
    Pwn30,
    /// Albanet nëpërmjet Open Multilingual Wordnet
    OmwAls,
    /// ConceptNet 5.7 (prefiks i pjesshëm, skajet /c/sq/)
    ConceptNet57,
    /// Universal Dependencies — UD_Albanian-TSA
    UdSqTsa,
    /// Universal Dependencies — UD_Albanian-STAF
    UdSqStaf,
    /// Zgjerim lokal i etiketuar veçmas (jo burim i jashtëm, jo histori organike)
    LocalExtension,
    /// BabelNet 5.3 — vetëm shqisat shqipe HIGH_QUALITY jashtë AlbaNet-it.
    BabelNet53,
    /// Rregullat semantike të fjalëformimit të shqipes (tabela lokale me burime gramatikore).
    SqRules,
    /// LEXIN suedisht–shqip
    Lexin,
    /// Leksiku gramatikor shqip (uniparser)
    Uniparser,
    /// Wiktionary anglisht (hyrjet shqipe)
    KaikkiEn,
    /// Wiktionary gjermanisht (hyrjet shqipe)
    KaikkiDe,
    /// EuroVoc (BE)
    EuroVoc,
    /// Lexemat e Wikidata
    WdLexemes,
    /// Fjalët e huaja → shqip (OpenCovenant)
    Loanwords,
    /// Variantet dialektore (FjaloriShqip)
    Dialects,
    /// Nikollë Gazulli, «Fjalorth i ri» (1941): fjalë të rralla të Veriut, me shpjegim shqip
    /// (domen publik; OCR i filtruar).
    Gazulli1941,
    /// Gustav Meyer, «Etymologisches Wörterbuch der albanesischen Sprache» (1891; domen publik;
    /// OCR i dy skanimeve të krahasuara).
    Meyer1891,
    /// Leksik i shkruar nga modeli gjuhësor i projektit me kërkesë të pronarit, i
    /// verifikuar nga një kalim i dytë kundërshtues; një dëshmitar, jo autoritet.
    ModelClaude,
    /// Shënim i pronarit të projektit (shpjegim popullor, i pavërtetuar; thuhet si i tillë).
    OwnerNote,
}

impl Source {
    pub fn code(self) -> &'static str {
        match self {
            Source::Pwn30 => "pwn30",
            Source::OmwAls => "omw_als",
            Source::ConceptNet57 => "conceptnet57",
            Source::UdSqTsa => "ud_sq_tsa",
            Source::UdSqStaf => "ud_sq_staf",
            Source::LocalExtension => "local_ext",
            Source::BabelNet53 => "babelnet53",
            Source::SqRules => "sq_rules",
            Source::Lexin => "lexin",
            Source::Uniparser => "uniparser",
            Source::KaikkiEn => "kaikki_en",
            Source::KaikkiDe => "kaikki_de",
            Source::EuroVoc => "eurovoc",
            Source::WdLexemes => "wd_lexemes",
            Source::Loanwords => "loanwords",
            Source::Dialects => "dialects",
            Source::Gazulli1941 => "gazulli1941",
            Source::Meyer1891 => "meyer1891",
            Source::ModelClaude => "model_claude",
            Source::OwnerNote => "owner_note",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Source::Pwn30 => "WordNet 3.0",
            Source::OmwAls => "Albanet / OMW",
            Source::ConceptNet57 => "ConceptNet 5.7",
            Source::UdSqTsa => "UD Albanian-TSA",
            Source::UdSqStaf => "UD Albanian-STAF",
            Source::LocalExtension => "Zgjerim lokal (i etiketuar)",
            Source::BabelNet53 => "BabelNet 5.3",
            Source::SqRules => "Rregullat e fjalëformimit të shqipes",
            Source::Lexin => "LEXIN suedisht–shqip",
            Source::Uniparser => "Leksiku gramatikor shqip (uniparser)",
            Source::KaikkiEn => "Wiktionary anglisht (hyrjet shqipe)",
            Source::KaikkiDe => "Wiktionary gjermanisht (hyrjet shqipe)",
            Source::EuroVoc => "EuroVoc (BE)",
            Source::WdLexemes => "Lexemat e Wikidata",
            Source::Loanwords => "Fjalët e huaja → shqip (OpenCovenant)",
            Source::Dialects => "Variantet dialektore (FjaloriShqip)",
            Source::Gazulli1941 => "Nikollë Gazulli, Fjalorth i ri (1941)",
            Source::Meyer1891 => "Gustav Meyer, Etymologisches Wörterbuch der albanesischen Sprache (1891)",
            Source::ModelClaude => "Leksik i modelit Claude (AI), i verifikuar nga një kalim i dytë",
            Source::OwnerNote => "Shënim i pronarit (shpjegim popullor, i pavërtetuar)",
        }
    }
    pub fn parse(s: &str) -> Option<Source> {
        match s {
            "pwn30" => Some(Source::Pwn30),
            "omw_als" => Some(Source::OmwAls),
            "conceptnet57" => Some(Source::ConceptNet57),
            "ud_sq_tsa" => Some(Source::UdSqTsa),
            "ud_sq_staf" => Some(Source::UdSqStaf),
            "local_ext" => Some(Source::LocalExtension),
            "babelnet53" => Some(Source::BabelNet53),
            "sq_rules" => Some(Source::SqRules),
            "lexin" => Some(Source::Lexin),
            "uniparser" => Some(Source::Uniparser),
            "kaikki_en" => Some(Source::KaikkiEn),
            "kaikki_de" => Some(Source::KaikkiDe),
            "eurovoc" => Some(Source::EuroVoc),
            "wd_lexemes" => Some(Source::WdLexemes),
            "loanwords" => Some(Source::Loanwords),
            "dialects" => Some(Source::Dialects),
            "gazulli1941" => Some(Source::Gazulli1941),
            "meyer1891" => Some(Source::Meyer1891),
            "model_claude" => Some(Source::ModelClaude),
            "owner_note" => Some(Source::OwnerNote),
            _ => None,
        }
    }
    pub const ALL: [Source; 6] = [
        Source::BabelNet53,
        Source::Pwn30,
        Source::OmwAls,
        Source::ConceptNet57,
        Source::UdSqTsa,
        Source::UdSqStaf,
    ];
}

/// Si u krijua një skaj.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// Marrëdhënie e botuar drejtpërdrejt nga burimi.
    Published,
    /// Lidhje e botuar nga OMW: lema shqipe → synset PWN 3.0 sipas offset-it.
    OmwOffsetMap,
    /// Metodë lokale e deklaruar: koncepti ConceptNet /c/sq/<lemë> lidhet me
    /// nyjën Word(sq, <lemë>) vetëm kur vargu i lemës përputhet saktë.
    /// Lidhja bëhet me FJALËN, kurrë me një kuptim.
    LemmaStringMatch,
    /// Anotim UD (lemë, tipare morfologjike, varësi sintaksore).
    UdAnnotation,
    /// Lidhje e deklaruar nga BabelNet: synset-i BabelNet ↔ synset WordNet 3.0
    /// përmes `senseKey` të WordNet dhe `index.sense` të Princeton-it.
    BnSenseKeyMap,
    /// Rregull i fjalëformimit të shqipes (id e rregullit te atributi `rule`).
    AlbanianRule,
    /// Hyrje e shkruar nga një model gjuhësor (njohuri e modelit, e verifikuar nga një kalim i dytë).
    ModelAuthored,
    /// E thënë nga pronari i projektit (shënim, jo burim i botuar).
    OwnerStated,
    /// Lidhje nga shpjegimi: fjala e përmendur në përkufizimin shqip, e kthyer në lemë me dëshmi
    /// (lema e fjalorit ose trajta e atestuar e një leme të vetme).
    GlossLemma,
}

impl Method {
    pub fn code(self) -> &'static str {
        match self {
            Method::Published => "published",
            Method::OmwOffsetMap => "omw_offset_map",
            Method::LemmaStringMatch => "lemma_string_match",
            Method::UdAnnotation => "ud_annotation",
            Method::BnSenseKeyMap => "bn_sensekey_map",
            Method::AlbanianRule => "albanian_rule",
            Method::ModelAuthored => "model_authored",
            Method::OwnerStated => "owner_stated",
            Method::GlossLemma => "gloss_lemma",
        }
    }
    pub fn parse(s: &str) -> Option<Method> {
        match s {
            "published" => Some(Method::Published),
            "omw_offset_map" => Some(Method::OmwOffsetMap),
            "lemma_string_match" => Some(Method::LemmaStringMatch),
            "ud_annotation" => Some(Method::UdAnnotation),
            "bn_sensekey_map" => Some(Method::BnSenseKeyMap),
            "albanian_rule" => Some(Method::AlbanianRule),
            "model_authored" => Some(Method::ModelAuthored),
            "owner_stated" => Some(Method::OwnerStated),
            "gloss_lemma" => Some(Method::GlossLemma),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Pos {
    Noun,
    Verb,
    Adj,
    Adv,
}

impl Pos {
    pub fn from_wn(c: &str) -> Option<Pos> {
        match c {
            "n" => Some(Pos::Noun),
            "v" => Some(Pos::Verb),
            "a" | "s" => Some(Pos::Adj),
            "r" => Some(Pos::Adv),
            _ => None,
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            Pos::Noun => "n",
            Pos::Verb => "v",
            Pos::Adj => "a",
            Pos::Adv => "r",
        }
    }
    pub fn label_sq(self) -> &'static str {
        match self {
            Pos::Noun => "emër",
            Pos::Verb => "folje",
            Pos::Adj => "mbiemër",
            Pos::Adv => "ndajfolje",
        }
    }
    pub fn from_upos(u: &str) -> Option<Pos> {
        match u {
            "NOUN" | "PROPN" => Some(Pos::Noun),
            "VERB" => Some(Pos::Verb),
            "ADJ" => Some(Pos::Adj),
            "ADV" => Some(Pos::Adv),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    /// Fjalë si formë leksikore (lemë) në një gjuhë. Nuk është kuptim.
    Word { lang: Lang, lemma: String },
    /// Kuptim (synset) i një burimi leksikor.
    Sense { source: Source, synset: String, pos: Pos },
    /// Koncept ConceptNet (term pa kuptim të caktuar; me POS opsional).
    Concept { uri: String },
    /// Fjali e anotuar (kontekst përdorimi) nga UD.
    Sentence { source: Source, sent_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    pub lang: Lang,
    pub text: String,
    pub source: Source,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub key: String,
    pub kind: NodeKind,
    /// Etiketa (lema) sipas gjuhës dhe burimit.
    pub labels: Vec<Text>,
    /// Përkufizime / shpjegime (gloss).
    pub glosses: Vec<Text>,
    /// Shembuj përdorimi të botuar nga burimi.
    pub examples: Vec<Text>,
}

/// Marrëdhëniet. Emri ruhet i dallueshëm sipas burimit: antonimia e PWN është
/// leksikore në anglisht dhe nuk barazohet me antoniminë shqipe të ConceptNet.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Rel {
    /// Word → Sense (fjala ka këtë kuptim)
    HasSense,
    /// Lidhje treguese PWN (simboli origjinal ruhet: "@", "~", "#p", "!", ...)
    Wn(String),
    /// Marrëdhënie ConceptNet (p.sh. "/r/Antonym")
    Cn(String),
    /// Concept → Word (metoda LemmaStringMatch)
    FormOf,
    /// Word → Sentence: fjala shfaqet në fjali me një formë dhe tipare
    OccursIn,
    /// Word(varur) → Word(kokë) brenda një fjalie UD; atributi `sent` jep kontekstin
    DependsOn(String),
    /// Marrëdhënie BabelNet (relationGroup: HYPERNYM, MERONYM, HOLONYM, ANTONYM)
    Bn(String),
    /// Fjala e prejardhur → fjala bazë sipas një rregulli të fjalëformimit (p.sh. NEGATION_OF)
    Deriv(String),
    /// Marrëdhënie semantike/leksikore ndërmjet DY FJALËVE shqipe nga një fjalor
    /// (SYNONYM, ANTONYM, HYPERNYM, HYPONYM, RELATED, COORDINATE, DERIVED_FROM,
    /// NATIVE_EQUIVALENT, DIALECT_VARIANT_OF, FORM_OF, COMPOUND_OF). Drejtimi: from → to.
    Sem(String),
    /// Të dhëna gramatikore të fjalës (vetë-lak: from == to); atributet: pos, gender,
    /// anim, trans, med, borr, flags.
    Lex,
}

impl Rel {
    pub fn code(&self) -> String {
        match self {
            Rel::HasSense => "has_sense".into(),
            Rel::Wn(s) => format!("wn:{s}"),
            Rel::Cn(s) => format!("cn:{s}"),
            Rel::FormOf => "form_of".into(),
            Rel::OccursIn => "occurs_in".into(),
            Rel::DependsOn(d) => format!("dep:{d}"),
            Rel::Bn(r) => format!("bnrel:{r}"),
            Rel::Deriv(r) => format!("deriv:{r}"),
            Rel::Sem(r) => format!("sem:{r}"),
            Rel::Lex => "lex".into(),
        }
    }
    pub fn parse(s: &str) -> Option<Rel> {
        match s {
            "has_sense" => Some(Rel::HasSense),
            "form_of" => Some(Rel::FormOf),
            "occurs_in" => Some(Rel::OccursIn),
            "lex" => Some(Rel::Lex),
            _ => match (
                s.strip_prefix("wn:"),
                s.strip_prefix("cn:"),
                s.strip_prefix("dep:"),
                s.strip_prefix("bnrel:"),
                s.strip_prefix("deriv:"),
                s.strip_prefix("sem:"),
            ) {
                (Some(x), _, _, _, _, _) => Some(Rel::Wn(x.into())),
                (_, Some(x), _, _, _, _) => Some(Rel::Cn(x.into())),
                (_, _, Some(x), _, _, _) => Some(Rel::DependsOn(x.into())),
                (_, _, _, Some(x), _, _) => Some(Rel::Bn(x.into())),
                (_, _, _, _, Some(x), _) => Some(Rel::Deriv(x.into())),
                (_, _, _, _, _, Some(x)) => Some(Rel::Sem(x.into())),
                _ => None,
            },
        }
    }
    /// Emri shqip i marrëdhënies për UI-në.
    pub fn label_sq(&self) -> String {
        match self {
            Rel::HasSense => "ka kuptimin".into(),
            Rel::FormOf => "formë e fjalës (përputhje lemë)".into(),
            Rel::OccursIn => "përdoret në fjalinë".into(),
            Rel::DependsOn(d) => format!("varësi sintaksore «{d}»"),
            Rel::Cn(r) => match r.as_str() {
                "/r/Antonym" => "e kundërt (ConceptNet)".into(),
                "/r/Synonym" => "sinonim (ConceptNet)".into(),
                "/r/RelatedTo" => "e lidhur (ConceptNet)".into(),
                other => format!("ConceptNet {other}"),
            },
            Rel::Wn(sym) => wn_symbol_label(sym).into(),
            Rel::Bn(r) => match r.as_str() {
                "HYPERNYM" => "hiperonim (lloj i) — BabelNet".into(),
                "MERONYM" => "meronim (ka pjesë) — BabelNet".into(),
                "HOLONYM" => "holonim (pjesë e) — BabelNet".into(),
                "ANTONYM" => "antonim — BabelNet".into(),
                other => format!("BabelNet {other}"),
            },
            Rel::Deriv(r) => format!("fjalëformim shqip: {r}"),
            Rel::Sem(r) => match r.as_str() {
                "SYNONYM" => "sinonim".into(),
                "ANTONYM" => "e kundërt".into(),
                "HYPERNYM" => "lloj i".into(),
                "HYPONYM" => "ka si lloj".into(),
                "DERIVED_FROM" => "e prejardhur nga".into(),
                "DIALECT_VARIANT_OF" => "variant dialektor i".into(),
                "NATIVE_EQUIVALENT" => "fjala shqipe për".into(),
                "REGIONAL_GLOSS" => "kuptim krahinor i Veriut (1941)".into(),
                other => format!("lidhje {other}"),
            },
            Rel::Lex => "të dhëna gramatikore".into(),
        }
    }
}

pub fn wn_symbol_label(sym: &str) -> &'static str {
    match sym {
        "@" => "hiperonim (lloj i)",
        "@i" => "hiperonim instance",
        "~" => "hiponim",
        "~i" => "hiponim instance",
        "#m" => "holonim anëtari",
        "#s" => "holonim lënde",
        "#p" => "holonim pjese (pjesë e)",
        "%m" => "meronim anëtari",
        "%s" => "meronim lënde",
        "%p" => "meronim pjese",
        "!" => "antonim leksikor (anglisht)",
        "=" => "atribut",
        "+" => "lidhje derivative",
        ";c" => "domen teme",
        "-c" => "anëtar domeni teme",
        ";r" => "domen rajoni",
        "-r" => "anëtar domeni rajoni",
        ";u" => "domen përdorimi",
        "-u" => "anëtar domeni përdorimi",
        "*" => "implikim (entailment)",
        ">" => "shkak",
        "^" => "shih edhe",
        "$" => "grup foljor",
        "&" => "i ngjashëm",
        "<" => "pjesore e foljes",
        "\\" => "rrjedhim (pertainym)",
        _ => "lidhje WordNet",
    }
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: u32,
    pub to: u32,
    pub rel: Rel,
    pub source: Source,
    pub lang: Option<Lang>,
    pub method: Method,
    /// Atribute shtesë (p.sh. forma, tiparet, fjalia, fjalët e PWN për antoniminë).
    pub attrs: Vec<(String, String)>,
}

impl Edge {
    pub fn attr(&self, k: &str) -> Option<&str> {
        self.attrs.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
    }
}

#[derive(Default)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub by_key: HashMap<String, u32>,
    pub out: Vec<Vec<u32>>,
    pub inc: Vec<Vec<u32>>,
}

pub fn word_key(lang: Lang, lemma: &str) -> String {
    format!("w:{}:{}", lang.code(), lemma)
}
pub fn sense_key(source: Source, synset: &str) -> String {
    format!("s:{}:{}", source.code(), synset)
}
pub fn concept_key(uri: &str) -> String {
    format!("c:{uri}")
}
pub fn sentence_key(source: Source, id: &str) -> String {
    format!("u:{}:{}", source.code(), id)
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<u32> {
        self.by_key.get(key).copied()
    }

    pub fn node(&self, id: u32) -> &Node {
        &self.nodes[id as usize]
    }

    /// Krijon nyjën nëse mungon; kthen id-në. Nyja ekzistuese nuk mbishkruhet.
    pub fn ensure(&mut self, key: String, kind: NodeKind) -> u32 {
        match self.by_key.get(&key) {
            Some(id) => *id,
            None => {
                let id = self.nodes.len() as u32;
                self.nodes.push(Node {
                    key: key.clone(),
                    kind,
                    labels: Vec::new(),
                    glosses: Vec::new(),
                    examples: Vec::new(),
                });
                self.out.push(Vec::new());
                self.inc.push(Vec::new());
                self.by_key.insert(key, id);
                id
            }
        }
    }

    pub fn ensure_word(&mut self, lang: Lang, lemma: &str) -> u32 {
        self.ensure(
            word_key(lang, lemma),
            NodeKind::Word { lang, lemma: lemma.to_string() },
        )
    }

    pub fn add_label(&mut self, id: u32, t: Text) {
        let n = &mut self.nodes[id as usize];
        match n.labels.contains(&t) {
            true => {}
            false => n.labels.push(t),
        }
    }

    /// Shton skajin; skaji identik (from,to,rel,source) nuk dyfishohet.
    pub fn add_edge(&mut self, e: Edge) -> u32 {
        let dup = self.out[e.from as usize].iter().find(|&&eid| {
            let x = &self.edges[eid as usize];
            x.to == e.to && x.rel == e.rel && x.source == e.source && x.attrs == e.attrs
        });
        match dup {
            Some(id) => *id,
            None => {
                let id = self.edges.len() as u32;
                self.out[e.from as usize].push(id);
                self.inc[e.to as usize].push(id);
                self.edges.push(e);
                id
            }
        }
    }

    pub fn out_edges(&self, id: u32) -> impl Iterator<Item = (u32, &Edge)> + '_ {
        self.out[id as usize].iter().map(move |&e| (e, &self.edges[e as usize]))
    }

    pub fn in_edges(&self, id: u32) -> impl Iterator<Item = (u32, &Edge)> + '_ {
        self.inc[id as usize].iter().map(move |&e| (e, &self.edges[e as usize]))
    }

    /// Etiketa e parë në gjuhën e kërkuar.
    pub fn label_in(&self, id: u32, lang: Lang) -> Option<&str> {
        self.node(id).labels.iter().find(|t| t.lang == lang).map(|t| t.text.as_str())
    }

    /// Etiketë e shkurtër për ekran: shqip kur ekziston, ndryshe anglisht me
    /// shenjën e gjuhës — kurrë anglisht e paraqitur si shqip.
    pub fn display_label(&self, id: u32) -> (String, Lang) {
        let n = self.node(id);
        match &n.kind {
            NodeKind::Word { lang, lemma } => (lemma.clone(), *lang),
            NodeKind::Concept { uri } => (concept_term(uri), uri.split('/').nth(2).and_then(Lang::parse).unwrap_or(Lang::En)),
            NodeKind::Sentence { sent_id, .. } => (sent_id.clone(), Lang::Sq),
            NodeKind::Sense { .. } => match (self.label_in(id, Lang::Sq), self.label_in(id, Lang::En)) {
                (Some(sq), _) => (sq.to_string(), Lang::Sq),
                (None, Some(en)) => (en.to_string(), Lang::En),
                (None, None) => (n.key.clone(), Lang::En),
            },
        }
    }

    pub fn gloss_in(&self, id: u32, lang: Lang) -> Option<&Text> {
        self.node(id).glosses.iter().find(|t| t.lang == lang)
    }

    /// Kuptimet e një fjale (skajet HasSense).
    pub fn senses_of(&self, word: u32) -> Vec<(u32, &Edge)> {
        self.out_edges(word)
            .filter(|(_, e)| e.rel == Rel::HasSense)
            .map(|(_, e)| (e.to, e))
            .collect()
    }

    /// Fjalët shqipe që e kanë këtë kuptim.
    pub fn words_of_sense(&self, sense: u32, lang: Lang) -> Vec<u32> {
        self.in_edges(sense)
            .filter(|(_, e)| e.rel == Rel::HasSense)
            .map(|(_, e)| e.from)
            .filter(|&w| match &self.node(w).kind { NodeKind::Word { lang: l, .. } => *l == lang, _ => false })
            .collect()
    }
}

/// "/c/sq/bardhë/a" → "bardhë"
pub fn concept_term(uri: &str) -> String {
    uri.split('/').nth(3).unwrap_or(uri).replace('_', " ")
}

/// "/c/sq/bardhë/a" → Some(Pos::Adj)
pub fn concept_pos(uri: &str) -> Option<Pos> {
    uri.split('/').nth(4).and_then(Pos::from_wn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_and_sense_have_distinct_identities() {
        let mut g = Graph::new();
        let w = g.ensure_word(Lang::Sq, "zjarr");
        let s1 = g.ensure(
            sense_key(Source::Pwn30, "07302836-n"),
            NodeKind::Sense { source: Source::Pwn30, synset: "07302836-n".into(), pos: Pos::Noun },
        );
        let s2 = g.ensure(
            sense_key(Source::Pwn30, "14842847-n"),
            NodeKind::Sense { source: Source::Pwn30, synset: "14842847-n".into(), pos: Pos::Noun },
        );
        for s in [s1, s2] {
            g.add_edge(Edge {
                from: w,
                to: s,
                rel: Rel::HasSense,
                source: Source::OmwAls,
                lang: Some(Lang::Sq),
                method: Method::OmwOffsetMap,
                attrs: vec![],
            });
        }
        assert_ne!(w, s1);
        assert_ne!(s1, s2);
        assert_eq!(g.senses_of(w).len(), 2);
    }

    #[test]
    fn duplicate_edges_are_not_created() {
        let mut g = Graph::new();
        let a = g.ensure_word(Lang::Sq, "a");
        let b = g.ensure_word(Lang::Sq, "b");
        let e = Edge {
            from: a,
            to: b,
            rel: Rel::Cn("/r/Antonym".into()),
            source: Source::ConceptNet57,
            lang: Some(Lang::Sq),
            method: Method::Published,
            attrs: vec![],
        };
        let x = g.add_edge(e.clone());
        let y = g.add_edge(e);
        assert_eq!(x, y);
        assert_eq!(g.edges.len(), 1);
    }

    #[test]
    fn rel_code_roundtrip() {
        for r in [
            Rel::HasSense,
            Rel::Wn("@".into()),
            Rel::Cn("/r/Antonym".into()),
            Rel::FormOf,
            Rel::OccursIn,
            Rel::DependsOn("nsubj".into()),
            Rel::Bn("HYPERNYM".into()),
            Rel::Deriv("NEGATION_OF".into()),
            Rel::Sem("SYNONYM".into()),
            Rel::Lex,
        ] {
            assert_eq!(Rel::parse(&r.code()), Some(r));
        }
    }
}
