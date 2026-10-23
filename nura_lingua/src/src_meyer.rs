//! Gustav Meyer, «Etymologisches Wörterbuch der albanesischen Sprache» (1891, domen publik):
//! dy skanime OCR të krahasuara, me drejtshkrimin e Meyer-it të kthyer në atë të sotëm.

use crate::graph::Graph;
use crate::import::ImportReport;
use crate::morph::Morph;

/// Skanimi më i plotë dhe skanimi i dytë (për krahasim).
pub const SCANS: [&str; 2] = [
    "raw/meyer1891/etymologischesw01meyegoog_djvu.txt",
    "raw/meyer1891/etymologischesw00meyegoog_djvu.txt",
];

pub fn import(_g: &mut Graph, _m: &Morph, _primary: &str, _second: &str, _rep: &mut ImportReport) {}
