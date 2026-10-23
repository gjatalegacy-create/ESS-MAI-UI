//! Importuesi i OpenCovenant/loanwords (MIT): fjalë e huaj → njëvlerëse shqipe.
//!
//! TSV (nga nura_fetch): `fjala_e_huaj ⇥ njëvlerësja ⇥ origjina ⇥ fjala_burimore`.
//! - `Sem("NATIVE_EQUIVALENT")` nga fjala e huaj te secila njëvlerëse, me `("origin", gjuha)`
//!   dhe `("etymon", fjala burimore)` — kjo e fundit është tekst i gjuhës së huaj dhe
//!   ruhet vetëm si atribut, kurrë si fjalë shqipe.
//! - Fjala e huaj lidhet vetëm kur ekziston tashmë si nyje `Word` ose kur është një fjalë e
//!   vetme (atëherë krijohet).
//! - Njëvlerësja «i/e X» ruhet si «X» me `("article", …)` vetëm kur «X» ekziston tashmë si
//!   fjalë; përndryshe mbetet frazë e plotë (p.sh. «i jep përparësi»).
//! - Kujdes: shumë çelësa të burimit janë forma të lakuara/zgjedhuara («abandonoi» → «braktisi»);
//!   burimi nuk i dallon nga lemat, prandaj importohen siç janë.
use crate::graph::*;
use crate::import::ImportReport;
use crate::morph::Morph;

pub const TSV: &str = "raw/loanwords/loanwords-sq.tsv";

fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// «i përshtatshëm» → («përshtatshëm», Some("i")) vetëm kur «përshtatshëm» është fjalë e njohur.
fn native_target(g: &Graph, alt: &str) -> (String, Option<&'static str>) {
    let toks: Vec<&str> = alt.split(' ').collect();
    let art = match toks.as_slice() {
        ["i", x] => Some((*x, "i")),
        ["e", x] => Some((*x, "e")),
        _ => None,
    };
    match art.map(|(x, a)| (x, a, g.get(&word_key(Lang::Sq, x)).is_some())) {
        Some((x, a, true)) => (x.to_string(), Some(a)),
        _ => (alt.to_string(), None),
    }
}

pub fn import(g: &mut Graph, _morph: &mut Morph, text: &str, rep: &mut ImportReport) -> usize {
    let (mut edges, mut skipped, mut rows_n) = (0usize, 0usize, 0usize);
    let mut loans = std::collections::HashSet::new();
    for l in text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let c: Vec<&str> = l.split('\t').collect();
        rows_n += 1;
        let (loan, alt) = (norm(c.first().copied().unwrap_or("")), norm(c.get(1).copied().unwrap_or("")));
        let origin = c.get(2).map(|s| s.trim()).unwrap_or("");
        let etymon = c.get(3).map(|s| s.trim()).unwrap_or("");
        let usable = !loan.is_empty() && !alt.is_empty() && loan != alt;
        let exists = g.get(&word_key(Lang::Sq, &loan)).is_some();
        match (usable, exists || !loan.contains(' ')) {
            (true, true) => {}
            _ => {
                skipped += 1;
                continue;
            }
        }
        let from = g.ensure_word(Lang::Sq, &loan);
        let (target, article) = native_target(g, &alt);
        let to = g.ensure_word(Lang::Sq, &target);
        let mut attrs: Vec<(String, String)> = Vec::new();
        match origin.is_empty() {
            true => {}
            false => attrs.push(("origin".into(), origin.to_string())),
        }
        match etymon.is_empty() {
            true => {}
            false => attrs.push(("etymon".into(), etymon.to_string())),
        }
        match article {
            Some(a) => attrs.push(("article".into(), a.into())),
            None => {}
        }
        attrs.push(("src".into(), "opencovenant-loanwords".into()));
        let before = g.edges.len();
        g.add_edge(Edge {
            from,
            to,
            rel: Rel::Sem("NATIVE_EQUIVALENT".into()),
            source: Source::Loanwords,
            lang: Some(Lang::Sq),
            method: Method::Published,
            attrs,
        });
        match g.edges.len() > before {
            true => {
                edges += 1;
                loans.insert(from);
            }
            false => {}
        }
    }
    rep.note(format!(
        "loanwords: {rows_n} çifte në TSV; {edges} lidhje NATIVE_EQUIVALENT nga {} fjalë të huaja; {skipped} çifte të anashkaluara (bosh / të njëjta / frazë e panjohur)",
        loans.len()
    ));
    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &str = "# loanwords-sq.tsv — test\n\
abandonoj\tbraktis\tenglish\tabandon\n\
bekgraund\tsfond\tenglish\tbackground\n\
bekgraund\tprapavijë\tenglish\tbackground\n\
adekuat\ti përshtatshëm\titalian\tadeguato\n\
prioritarizon\ti jep përparësi\tenglish\tprioritize\n\
kod bar\tkod vijor\tenglish\tbarcode\n\
linku\tlinku\tenglish\tlink\n\
pa-tabulim\n";

    fn ne(g: &Graph, a: &str, b: &str) -> bool {
        let (a, b) = (g.get(&word_key(Lang::Sq, a)).unwrap(), g.get(&word_key(Lang::Sq, b)).unwrap());
        g.out_edges(a).any(|(_, e)| e.to == b && e.rel == Rel::Sem("NATIVE_EQUIVALENT".into()))
    }

    #[test]
    fn imports_native_equivalents() {
        let mut g = Graph::new();
        g.ensure_word(Lang::Sq, "përshtatshëm");
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        let n = import(&mut g, &mut m, FIX, &mut r);
        assert_eq!(n, 5);
        assert!(ne(&g, "abandonoj", "braktis"));
        assert!(ne(&g, "bekgraund", "sfond"));
        assert!(ne(&g, "bekgraund", "prapavijë"));
        // Nyja e mbiemrit ndahet vetëm kur mbiemri njihet.
        assert!(ne(&g, "adekuat", "përshtatshëm"));
        assert!(ne(&g, "prioritarizon", "i jep përparësi"));
        // Frazë e huaj e panjohur nuk krijohet; çifti identik nuk lidhet.
        assert!(g.get(&word_key(Lang::Sq, "kod bar")).is_none());
        assert!(g.get(&word_key(Lang::Sq, "linku")).is_none());
        // Fjala burimore e huaj nuk bëhet fjalë shqipe.
        assert!(g.get(&word_key(Lang::Sq, "abandon")).is_none());
        let e = g.edges.iter().find(|e| e.attr("etymon") == Some("abandon")).unwrap();
        assert_eq!(e.attr("origin"), Some("english"));
        assert_eq!(e.source, Source::Loanwords);
    }

    #[test]
    fn existing_phrase_is_linked() {
        let mut g = Graph::new();
        g.ensure_word(Lang::Sq, "kod bar");
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        assert_eq!(import(&mut g, &mut m, "kod bar\tkod vijor\tenglish\tbarcode\n", &mut r), 1);
    }

    #[test]
    fn empty_and_partial() {
        let mut g = Graph::new();
        let mut m = Morph::default();
        let mut r = ImportReport::default();
        assert_eq!(import(&mut g, &mut m, "", &mut r), 0);
        assert_eq!(import(&mut g, &mut m, "# x\nvetëm\n\t\n", &mut r), 0);
        assert!(g.nodes.is_empty());
    }
}
