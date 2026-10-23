# Validation — 23 October 2026

Executed on Windows in an isolated publication snapshot, using the existing Cargo.lock and cached dependencies. The original UI and the canonical/private runtime were not modified. No test bodies, expectations or ignore annotations were changed.

| Check | Observed result |
|---|---|
| `cargo run --release --locked --offline -p nura_lingua --bin nura_import` | PASS; rebuilt graph from the public-data subset |
| `cargo build --release --locked --offline --workspace` | PASS |
| `cargo test --release --locked --offline --workspace --no-fail-fast` | 429 passed, 2 failed, 0 ignored |
| Local HTTP UI smoke check | PASS |
| Source/resource comparison | 123 original files; only four documented comment/header edits |
| Distributed corpus byte checks | 51 files match their SHA-256 records |

The full suite is **not green** if any failures are shown. Corpus-dependent expectations are retained even though the public data subset excludes BabelNet and Gazulli. The exact failing names are:

- `rare_northern_words_are_explained_from_gazulli_1941`
- `translations_follow_the_word_not_a_broader_synset`

Removing a restricted corpus is a distribution boundary, not a correction to test expectations. These failures are reported without claiming that each is caused solely by the excluded data. Detailed assertion messages are in `validation/cargo_test_all.log`. No independent reviewer validation is claimed. Compiler environment messages are preserved in the logs.

The UI smoke check runs the local server only. It disables product discovery using a nonexistent root and does not invoke the private ESS-MAI product. The regenerated graph contains 305,687 nodes, 954,244 edges and 202,811 morphology entries. Graph size and test counts do not establish scientific correctness.

GitHub users need the matching HF `data/` snapshot to reproduce corpus-dependent tests. Compilation needs the platform toolchain and Tauri dependencies. Offline Cargo requires dependencies to have been fetched beforehand.
