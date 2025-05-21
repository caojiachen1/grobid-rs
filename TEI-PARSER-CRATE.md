# Should `tei-parser` Live in Its Own Crate?

In a word, **yes** — if you want clear boundaries, re-usability, and easier versioning. Here's a framework for deciding.

---

## Comparison: Separate `tei-parser` Crate vs. Keep Inside `grobid-rs`

| Question                | Separate `tei-parser` crate                                 | Keep inside `grobid-rs`                |
|-------------------------|-------------------------------------------------------------|----------------------------------------|
| Who else might use it?  | Other Rust projects that need TEI → Rust structs but not JNI/Grobid. | Only grobid-rs consumers.              |
| Release cadence         | Can evolve independently: iterate parsing logic, bump semver without touching JNI side. | Coupled to Grobid bindings; every change triggers full release. |
| Compile times / features| Tiny crate (quick-xml + serde), single target, compiles fast; grobid-rs users get it via Cargo feature. | Monolithic build; every change recompiles full JNI tree. |
| Testing                 | Pure-Rust tests (no JVM) run in seconds; easier fuzzing.    | Tests require JNI env or mocks.        |
| Workspace complexity    | Adds one package to workspace; publish optional crate to crates.io. | Simpler repo tree, but harder to reuse elsewhere. |
| API ergonomics          | `tei_parser::parse_header(&str) -> HeaderMetadata` etc. Clear, no Grobid context. | Parsing functions buried under `grobid_rs::converters::tei`. |
| Long-term goal          | Facilitates the "Grobid-less" native pipeline (PDF→TEI→struct). | Ties parser's evolution to existing JNI flow. |

---

## Suggested Architecture

```
workspace/
├─ tei-parser/          # new crate
│   ├─ src/
│   │   ├─ lib.rs       # public parse_* fns
│   │   ├─ models.rs    # Serde structs (or re-export shared models)
│   │   └─ error.rs
│   └─ Cargo.toml
├─ grobid-rs/           # existing crate
│   └─ Cargo.toml       # add `tei-parser = { path = "../tei-parser", version = "0.x" }`
└─ xtask/               # build helpers
```

- **Models**
  - Option A: keep structs in `tei-parser` and re-export in `grobid_rs::{models::*}` via `pub use`.
  - Option B: create a third small `grobid-models` crate shared by both (cleaner if you also want JSON-only clients).
- **Features & deps**
  - `tei-parser` is `no_std`-friendly if you ever need it (just gate quick-xml allocations).
  - `grobid-rs` adds `tei-parser` under a default feature ("tei"), so downstream users can disable it if they only want JNI raw TEI.
- **Versioning**
  - Adopt semver alignment: `grobid-rs 0.4` depends on `tei-parser ^0.2`. Breaking changes to TEI structs bump the minor of `tei-parser`, but `grobid-rs` only needs a patch release if its public API didn't change.

---

## Migration Steps

1. **Extract code:** move `converters/tei.rs`, `models/*` into new crate.
2. **Set up CI:** build & test both crates in the workspace matrix.
3. **Update docs:** explain that `tei-parser` is standalone.
4. **Publish (optional):** `cargo publish --dry-run` to ensure metadata.
5. **Deprecation note:** in `grobid_rs::converters`, keep thin shims that delegate to the new crate; mark as `#[deprecated]` to ease transition.

---

## When *Not* to Split
- If you expect the TEI schema to diverge per Grobid version and want tight lock-step.
- If maintaining two crates feels overhead for your team size.
- If consumers will always pull Grobid anyway (e.g., CLI users who never call the parser directly).

---

## Recommendation

Given your roadmap includes:
- Native pipeline (PDFium/PDFAlto → TEI → Rust structs)
- HTTP micro-service that may serve JSON without JNI
- Desire for plugin-style modularity

…a dedicated `tei-parser` crate future-proofs you. It's lightweight to spin up now and pays dividends once you start experiments beyond the Java engine.

Let me know if you want a concrete extraction PR checklist or cargo config snippet and I'll draft it up. 