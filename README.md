# ESS-MAI UI — Nura Legacy

A standalone Rust/Tauri 2 desktop interface and Albanian linguistic graph workspace from the ESS-MAI project, maintained by **Bledar Gjata / Gjata Legacy**.

The UI includes the Albanian grammar and graph engine, importers, source download tools, desktop/web assets, and an optional bridge to an independently installed ESS-MAI product. It does not include the private AIGovern-Cyber runtime or the author's conversations and Vault.

## Distribution

- [GitHub source distribution](https://github.com/gjatalegacy-create/ESS-MAI-UI): all UI source, assets, tests, fixed source metadata and reproduction instructions; bulk corpora are supplied separately.
- [Hugging Face full UI distribution](https://huggingface.co/datasets/gjata-legacy/ess-mai-ui-nura-legacy): the same complete UI code, plus the distributable corpus snapshot and a rebuilt local graph index. This is a downloadable application archive, not a hosted inference service.

The complete UI source is present in both distributions. API keys, personal state, build caches, old binaries and third-party data without verified public redistribution rights are excluded. BabelNet is not redistributed. The Gazulli 1941 corpus is also excluded pending verification of redistribution rights across jurisdictions. These exclusions reduce available lexical coverage; they do not remove their import implementations. See [distribution scope](DISTRIBUTION.md) and [test results](VALIDATION.md).

## Build and run

Windows requires a Rust MSVC toolchain, the C++ build tools and WebView2. Other platforms require their Tauri 2 system dependencies and were not validated in this publication.

```powershell
cargo fetch --locked
cargo build --release --locked --offline --workspace
```

With the HF package's data snapshot, rebuild its index locally to reproduce it:

```powershell
cargo run --release --locked --offline -p nura_lingua --bin nura_import
cargo run --release --locked --offline -p ui_ess_mai
```

The optional local browser mode uses the same dispatcher:

```powershell
cargo run --release --locked --offline -p ui_ess_mai -- --serve 7878
```

Then open `http://127.0.0.1:7878/`. It intentionally rejects remote Host/Origin values. For a GitHub checkout, copy the `data/` folder from the matching HF snapshot first. The downloader can instead acquire upstream resources, but current upstream files may differ from the pinned snapshot. Downloads require network access; Cargo's `--offline` only controls dependency resolution.

```powershell
cargo test --release --locked --offline --workspace --no-fail-fast
```

Tests are preserved, including tests that require excluded corpora. Read `VALIDATION.md` for actual results and dataset requirements; no tests were marked ignored to create a green result.

## Components and boundaries

| Component | Role |
|---|---|
| `app/` | Tauri desktop interface, localhost service, product bridge and display assets |
| `nura_lingua/` | Graph, Albanian grammar, import, query and expression engine; no external Rust dependencies |
| `nura_fetch/` | Upstream resource download and source-metadata recording |
| `inherited/emotional_command.rs` | ESS-MAI emotional-command contract retained from the supplied UI |
| `model/` | Fixed supplemental lexical resources; separate source identity and witness rules |
| `rules/` | Albanian word-formation rules |
| `data/SOURCES.lock` | Corpus URLs, versions, licenses, sizes and SHA-256 values |

The optional product bridge uses `ESSMAI_PRODUCT_ROOT` or `data/product_root.txt`. Commands such as `provo` require separately installed compatible Light/Quantum/Shadow executables. Their absence does not install or expose the private product. The inherited local fallback path remains for behavioral compatibility; configure your own root explicitly. The publication process does not execute product commands.

SHA-256 checks establish byte integrity against the recorded snapshot; they do not independently prove factual correctness or authority. This interface and its linguistic graph are not a general-purpose proof engine or an independently certified security product.

## License, provenance and review

Project code is published under [Apache-2.0](LICENSE), the same license as the public ESS-MAI repository. External datasets retain their own terms and attribution in [THIRD_PARTY.md](THIRD_PARTY.md) and `SOURCES.lock`; the root license does not relicense those datasets. See [PROVENANCE.md](PROVENANCE.md) for the source relationship and editorial-only publication changes.

Independent reproduction and specific criticism are welcome in [GitHub issues](https://github.com/gjatalegacy-create/ESS-MAI-UI/issues). Report the commit, operating system, command, input, observed result and expected result. A successful build alone is not validation of the architectural theories.

[Author, publications and project accounts](LINKS.md) · [Citation metadata](CITATION.cff)
