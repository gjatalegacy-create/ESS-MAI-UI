# Distribution scope

This release snapshots the supplied `ui_ess_mai` workspace. The working source and the canonical ESS-MAI source were not edited during packaging.

GitHub includes all UI source, tests, assets, supplemental lexicon, rules and metadata. HF additionally includes 14 source groups from the local corpus snapshot and a newly generated graph index. The original index is not copied, because it also contained excluded data.

Excluded from both: authentication material; BabelNet cache/data; personal product state, Vault, cases, handoff and local paths files; old executables and WebView2 DLL; compiler output; generated Tauri schemas; internal execution report; machine-specific nightly scheduler script and audit intermediates. Gazulli 1941 data is held back until cross-jurisdiction redistribution rights are checked. No compatible private product runtime is included or required to build the UI.

Optional importer code for excluded resources remains. A user who has the necessary rights may acquire those resources independently. No credentials are supplied. Do not upload your API keys, product_root.txt, Vault or conversation state.
