//! Etimologjia e fjalëve shqipe sipas shablloneve të strukturuara të prejardhjes në Wiktionary-t
//! anglisht (kaikki.org, CC BY-SA): e trashëguar (protoshqip, indoevropiane), e huazuar (gjuha
//! dhe forma burimore), e prejardhur brenda shqipes, e pasigurt.

use crate::graph::Graph;
use crate::import::ImportReport;

/// Nxjerrja e prejardhjeve nga `nura_fetch --only kaikki_en_etym`.
pub const TSV: &str = "raw/kaikki_en/kaikki_en-sq.etym.tsv";

pub fn import(_g: &mut Graph, _text: &str, _rep: &mut ImportReport) {}
