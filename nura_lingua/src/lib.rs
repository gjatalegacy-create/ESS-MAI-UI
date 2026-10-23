//! nura_lingua — bërthama e ui_ess_mai.
//!
//! Tri pjesë, me role të ndara:
//! - `graph` + `import` + `store` + `morph`: grafi gjuhësor (fjalë ≠ kuptim),
//!   i ndërtuar nga WordNet 3.0, Albanet/OMW, ConceptNet 5.7 (prefiks) dhe UD shqip;
//! - `emotion`: makina e gjendjeve shprehëse (funksion i modeluar ndërveprimi,
//!   jo pretendim për ndjenja);
//! - `query` + `expression` + `layout`: kolapsi i fjalës (i₀ → Xi/Yi),
//!   plani i shprehjes dhe realizimi shqip, paraqitja e procesit.
//!
//! Disiplina e trashëguar nga ESS-MAI: pa if/else (vetëm match), masa me pikë
//! fikse 0..=10_000, pa rrjet dhe pa model të jashtëm gjatë ekzekutimit.

pub mod biseda;
pub mod derive;
pub mod discourse;
pub mod emotion;
pub mod expression;
pub mod fjalet;
pub mod gramatika;
pub mod graph;
pub mod import;
pub mod layout;
pub mod morph;
pub mod oratoria;
pub mod query;
pub mod sha256;
pub mod src_lexin;
pub mod src_uniparser;
pub mod src_kaikki_en;
pub mod src_kaikki_de;
pub mod src_eurovoc;
pub mod src_wd_lexemes;
pub mod src_loanwords;
pub mod src_gazulli;
pub mod src_model;
pub mod src_owner;
pub mod verb;
pub mod sentence;
pub mod src_dialects;
pub mod src_commonsense;
pub mod src_etymology;
pub mod src_meyer;
pub mod src_perkufizime;
pub mod store;

pub const MASS_SCALE: u32 = 10_000;
