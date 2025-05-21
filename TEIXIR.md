# TEIXIR: Grobid-in-Rust – Native Rust AI Parser and Ingestor

## Vision

A 100% Rust, JVM-free replacement for Grobid: fast, modular, and easy to deploy. This document outlines where we are, what blocks us, and how we can incrementally build a native Rust pipeline for scholarly document parsing.

---

## Track Progress

| Step | Current Status | What Still Blocks Us | Early Milestones (Δ time) |
|------|---------------|----------------------|--------------------------|
| 1. PDF → ALTO layout extraction | We still shell out to pdfalto (C++) just like vanilla Grobid. | pdfalto's code base is huge and GPL-3; rewriting it is a multi-month undertaking. We can, however, call Poppler + Tesseract libraries directly via Rust FFI and avoid an intermediate process. | Weekend spike: write a thin Rust wrapper over Poppler's page iterator, emit TextBox { text, bbox } JSON instead of ALTO. |
| 2. ML feature extraction | Grobid's "CRFFeature" Java classes parse the ALTO, compute local features into a dense sparse matrix. | A straight port is mechanical but tedious. 90% is string processing & regexes – perfect for Rust (faster + zero-copy). | 2-week sprint: re-implement header-feature generator (HeaderTrainingDatacator) in Rust and fuzz-test output against Grobid. |
| 3. Sequence-labelling models | Grobid uses Wapiti (CRF) via JNI; the model files are plain text. | Wapiti has a clean C API → trivial to link from Rust; we lose nothing here. Option B: swap in a lightweight transformer (LaBSE / LayoutLM-tiny) via ggml. | Couple of evenings: wrap Wapiti C API with safe Rust, load existing model, run inference on the features from (2). |
| 4. Post-processing to TEI / JSON | The Java layer walks Wapiti tags → TEI DOM via JDOM. | Writing TEI with quick-xml is already in progress (our new converters). | Already underway: finish the TEI → serde structs path, then create the reverse serializer for TEI if we still want XML; JSON is trivial. |
| 5. Orchestration / concurrency | Grobid spins Jetty and manages worker pools in Java. | We already have a Rust engine pool around JNI; porting that to pure Rust is mostly deleting code. | 1 day once (1-4) work: swap the JNI pool for direct Rust components, reuse our existing EnginePool abstraction. |
| 6. Training pipeline | Grobid's trainer is all Gradle/Java. | We only need to call Wapiti's --train; can replicate feature generation from step (2). | Optional – not required for inference MVP. |

---

## Why It's Plausible

- **Zero heavy dependencies** – The only "big" native libs we'd need are Poppler and Wapiti; both have mature C APIs.
- **Incremental path** – We can replace Grobid module-by-module and keep the existing CLI/HTTP API stable.
- **Performance upside** – No JVM start-up (~2-3 s cold), less RSS, and we avoid the JNI marshalling overhead.
- **Licensing** – Everything stays Apache 2.0 / MIT except Poppler (GPL) — but we already ship pdfalto which brings the same licence constraint.

---

## Concrete "Green-field" Plan (Rough Order)

1. **Proof-of-concept (⏱ ~3 days)**
    - Parse a PDF with Poppler → emit Vec<TextBox> JSON.
    - Feed existing Grobid Wapiti header model via new Rust wrapper.
    - Produce numbered tag sequence to prove parity with Grobid's processHeader.
2. **Header pipeline parity (⏱ ~2 weeks)**
    - Port feature generation rust-side.
    - Validate on 100-PDF canary set — goal: ≤ 0.5% token diff vs Java.
3. **CLI drop-in (⏱ ~1 week)**
    - Introduce --native flag in grobid-cli; route to native path when models present.
    - Ship experimental static build artefact via GitHub release.
4. **Full-text & refs (⏱ 1-2 months, parallelisable)**
    - Repeat feature-gen port, maybe switch long-range tasks to a small transformer for quality boost.
5. **HTTP server rewrite (⏱ 1 week once above ready)**
    - Replace Jetty servlet with axum service that calls the native engine pool.

---

## Near-term Action Items

| When         | Task                                 | Owner   | Notes                                  |
|--------------|--------------------------------------|---------|----------------------------------------|
| Next sprint  | Spike Poppler wrapper + Wapiti FFI crate | You? me? | get cargo test producing a tag dump     |
| After spike  | Decide: keep pdfalto or go Poppler-only? | team    | pdfalto better glyph clustering but bigger |
| In parallel  | Finish TEI ⇋ serde converters        | ongoing | required regardless of backend         |
| Quarter goal | Header & references parity demo; measure CPU/RAM vs JVM |         | sets go/no-go for full rewrite         |

---

## Risks / Unknowns

- **Fonts & Unicode quirks** – pdfalto handles CMap intricacies Poppler sometimes misses; quality evaluation needed.
- **Training** – If we diverge from exact Grobid features, we may need to re-train; but model files are small, training is fast.
- **Community ecosystem** – Grobid users rely on TEI; any surface incompatibility erodes adoption. Keep gold-standard tests.

---

## TL;DR – A 100% Rust, JVM-free Grobid is Realistic and Incremental

The fastest path is: PDF text extraction → re-use existing Wapiti models → re-emit the same TEI/JSON.
If you're up for a weekend hack, start with step #1 (Poppler + Wapiti wrapper) and wire that into our current CLI under a hidden flag. What do you think? 