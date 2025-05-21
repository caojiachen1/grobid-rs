# High-Leverage / High-ROI Bets to Line-up Next for Grobid-RS

Below is a ranked list of high-impact, high-leverage initiatives for the Grobid-RS ecosystem. Each section details the motivation, engineering effort, implementation plan, quick wins, risks, and how it fits into the broader roadmap.

---

## Summary Table

| Rank | Theme | Why it matters | Rough Effort | Quick Wins |
|------|-------|----------------|--------------|------------|
| ① | Model Modernisation (LayoutLM-tiny / OnnxRuntime) | Boosts F-score, enables multilingual, JVM-free, low-RAM inference | 2–3 weeks PoC | Drop-in .onnx model, header task first |
| ② | Static, Cross-arch Release Bundles (musl + Zig + mold) | Zero system deps, instant install, fewer support issues | 1 week | `curl | tar | ./grobid-cli` story |
| ③ | WASM Lite (web workers / Edge / Cloudflare) | Opens up front-end/serverless, fast client-side parsing | 2–4 weeks R&D | Header-only MVP |
| ④ | Streaming / Pipe-in Pipe-out CLI | Simplifies batch ETL, UNIX-friendly | 2–3 days | `cat *.pdf | grobid-cli header –stdin` |
| ⑤ | First-class Cloud Observability | Critical for prod ops, Prometheus/OTEL | 2 days | /metrics endpoint |
| ⑥ | Plugin Sandbox + gRPC | Extensible post-processing, custom plugins | 1 week | Host-side PoC |
| ⑦ | Auto-Tuner for JVM / Native Params | Optimal perf out-of-the-box, less config pain | 2 days | autotune.toml |
| ⑧ | Retrain-in-the-loop | Improves accuracy over time, leverages user feedback | 1 week | --save-errors + train wrapper |
| ⑨ | Integrated Table Detector | Major value-add for scientific PDFs | 2–3 weeks | Table region annotation |
| ⑩ | "Slim-No-Model" Mode for Edge | Enables SaaS/edge split, low-latency | 3 days | 1-MB stub client |

---

## ① Model Modernisation (LayoutLM-tiny / OnnxRuntime)
### Why it matters
Modern transformer models (e.g., LayoutLM-tiny) can significantly improve extraction accuracy, especially for multilingual and complex layouts. Moving away from CRF enables single-pass inference, lower latency, and removes JVM dependency. This unlocks new use-cases (e.g., serverless, edge) and reduces RAM usage.

### Engineering effort
- 2–3 weeks for a proof-of-concept (PoC) focused on the header extraction task.
- Involves integrating the `onnxruntime` crate, wiring up a tokenizer (e.g., `tokenizers`), and adapting the inference pipeline.

### Implementation plan
1. Select a pre-trained LayoutLM-tiny model and convert to ONNX format.
2. Integrate `onnxruntime` and `tokenizers` crates.
3. Build a Rust pipeline: PDF → tokens → model → tags.
4. Validate on a sample set of PDFs; compare F1 to CRF baseline.
5. Expose as a feature-flagged path in CLI.

### Quick wins / MVP
- Header-only extraction with ONNX model.
- CLI flag to select model backend.

### Risks / blockers
- ONNX model conversion quirks.
- Tokenizer alignment with model.
- Model size and inference speed on low-end hardware.

### Roadmap fit
- Foundation for future WASM, edge, and cloud-native deployments.

---

## ② Static, Cross-arch Release Bundles (musl + Zig + mold)
### Why it matters
A zero-dependency, static binary install story removes friction for new users and eliminates most environment-related support issues. This is a major driver for adoption and reliability.

### Engineering effort
- 1 week for initial setup and CI integration.
- Involves configuring musl builds, possibly using Zig and mold for cross-compilation.

### Implementation plan
1. Define artifact layout: `bin/`, `runtime/`, `models/`, `completions/`.
2. Choose between `cargo-dist` or custom `xtask dist` for packaging.
3. Set up CI matrix for all target platforms (Linux, macOS, Windows, ARM).
4. Test install and run on clean VMs/containers.

### Quick wins / MVP
- Provide a single tarball for each platform.
- Document install in README.

### Risks / blockers
- Cross-compiling JNI and native dependencies.
- Platform-specific quirks (e.g., macOS codesigning).

### Roadmap fit
- Unblocks broader adoption, especially in enterprise and cloud.

---

## ③ WASM Lite (web workers / Edge / Cloudflare)
### Why it matters
Enables client-side and serverless use-cases, such as browser-based parsing or edge compute (Cloudflare Workers). Opens up new user segments and enables pay-per-doc APIs.

### Engineering effort
- 2–4 weeks R&D for a header-only MVP.
- Requires WASM-compatible model (Wapiti or ONNX), and adapting PDF/text extraction for WASM.

### Implementation plan
1. Compile core pipeline to WASM (using wasm-pack or similar).
2. Adapt PDF/text extraction to WASM-safe APIs.
3. Integrate with web worker or Cloudflare Worker runtime.
4. Expose a simple JS API for header extraction.

### Quick wins / MVP
- Header-only WASM build.
- Demo in browser or on Cloudflare.

### Risks / blockers
- WASM compatibility of dependencies.
- Model inference speed in browser.

### Roadmap fit
- Enables new SaaS and front-end integrations.

---

## ④ Streaming / Pipe-in Pipe-out CLI
### Why it matters
UNIX-style streaming enables easy batch processing and integration into ETL/data pipelines. Reduces friction for data engineers and power users.

### Engineering effort
- 2–3 days for a basic implementation.
- Mostly async I/O and CLI protocol design.

### Implementation plan
1. Add CLI flags for `--stdin` and `--stdout` modes.
2. Implement async reading of PDF bytes from stdin.
3. Output results as JSON/TEI to stdout.
4. Document framing protocol for multi-PDF streams.

### Quick wins / MVP
- `cat *.pdf | grobid-cli header --stdin` works.

### Risks / blockers
- Handling large PDFs in memory.
- Framing protocol for multi-file streams.

### Roadmap fit
- Unblocks batch and pipeline use-cases.

---

## ⑤ First-class Cloud Observability
### Why it matters
Production users need metrics and tracing for monitoring, alerting, and scaling. Prometheus and OTEL are industry standards.

### Engineering effort
- 2 days to add metrics-exporter-prometheus and span instrumentation.

### Implementation plan
1. Integrate `metrics-exporter-prometheus` crate.
2. Add `/metrics` endpoint to Axum server.
3. Instrument key spans (PDF parse, model inference, etc.).
4. Document metrics for ops teams.

### Quick wins / MVP
- Prometheus scrapeable endpoint.

### Risks / blockers
- Ensuring low-overhead metrics.
- Security of metrics endpoint.

### Roadmap fit
- Required for production/enterprise adoption.

---

## ⑥ Plugin Sandbox + gRPC
### Why it matters
Allows third-parties to extend Grobid-RS with custom post-processing, e.g., citation graph builders, without forking the core. Enables a plugin ecosystem.

### Engineering effort
- 1 week for host-side PoC.
- Involves gRPC (tonic) and/or WASI/libloading for plugin loading.

### Implementation plan
1. Define plugin trait and gRPC proto.
2. Add plugin loader (libloading or WASI).
3. Expose plugin registration in CLI/config.
4. Document plugin API and safety constraints.

### Quick wins / MVP
- Load a simple plugin that post-processes extracted data.

### Risks / blockers
- Plugin sandboxing and safety.
- Versioning and ABI stability.

### Roadmap fit
- Unlocks ecosystem and custom workflows.

---

## ⑦ Auto-Tuner for JVM / Native Params
### Why it matters
Automatically tuning JVM and native parameters (heap, threads, mmap) ensures optimal performance out-of-the-box, reducing user frustration and support load.

### Engineering effort
- 2 days for a fun hack.

### Implementation plan
1. Add `grobid-cli autotune` command.
2. Run micro-benchmarks on first install.
3. Store results in `$XDG_CACHE_HOME/grobid-rs/autotune.toml`.
4. Use tuned params in future runs.

### Quick wins / MVP
- One-shot autotune command.

### Risks / blockers
- Benchmarking accuracy.
- Platform-specific tuning.

### Roadmap fit
- Improves user experience and performance.

---

## ⑧ Retrain-in-the-loop
### Why it matters
Enables continuous improvement by collecting and retraining on user corrections. Makes the system self-improving at sites with gold data.

### Engineering effort
- 1 week for initial implementation.

### Implementation plan
1. Add `--save-errors` flag to CLI.
2. Dump disagreeing TEI+PDF pairs for review.
3. Add `grobid-cli train` wrapper to re-feed corrected data to Wapiti.
4. Document retraining workflow.

### Quick wins / MVP
- Error dump and retrain commands.

### Risks / blockers
- Data privacy and management.
- Retraining stability.

### Roadmap fit
- Drives long-term accuracy improvements.

---

## ⑨ Integrated Table Detector
### Why it matters
Table extraction is a major pain point in scientific PDFs. Integrating a table detector (e.g., tabby, camelot-rs) behind the PdfProvider trait can dramatically improve value for research and enterprise users.

### Engineering effort
- 2–3 weeks for integration and tuning.

### Implementation plan
1. Evaluate table detection libraries (tabby, camelot-rs).
2. Integrate as a plugin or feature in PdfProvider.
3. Annotate <table> regions in output.
4. Validate on scientific PDF corpus.

### Quick wins / MVP
- Table region annotation in output.

### Risks / blockers
- Table detection accuracy.
- Library compatibility and performance.

### Roadmap fit
- Differentiator for scientific/academic use-cases.

---

## ⑩ "Slim-No-Model" Mode for Edge
### Why it matters
Enables a split architecture: lightweight client does PDF parsing, offloads ML inference to SaaS. Reduces latency and bandwidth, ideal for edge and mobile.

### Engineering effort
- 3 days for a stub client.

### Implementation plan
1. Build a 1-MB stub client that parses PDF and calls remote API for inference.
2. Expose as a CLI flag or config.
3. Document SaaS integration.

### Quick wins / MVP
- Working stub client.

### Risks / blockers
- API stability and security.
- Network reliability.

### Roadmap fit
- Enables SaaS/edge hybrid deployments.

---

## Suggested Sequencing

1. Static Release Bundles (②): Cheap, delivers instant DX joy.
2. Streaming CLI / Pipe Mode (④): Unblocks data-engineering adoption.
3. Model Modernisation (①): Medium lift but marquee payoff—start on a branch while ②/④ land.
4. Observability Hooks (⑤) and Auto-Tuner (⑦): Fold into the upcoming Axum rewrite.
5. Table Detector (⑨): Nice differentiator once core accuracy is solid.
6. Plugin / gRPC Layer (⑥): Makes sense after the PDF backend trait is bedded in.
7. WASM & Edge (③): Tackle after native path stabilises; piggy-backs on OnnxRuntime-web.
8. Retrain-in-loop (⑧) and Slim-No-Model (⑩): Longer-tail ecosystem pieces once you have real-world feedback.

---

## Quick Next Steps

- **Pick a champion for Static Bundles:** Define artifact layout (bin/, runtime/, models/, completions/). Decide whether to use cargo dist or hand-rolled xtask dist.
- **Frame a LayoutLM-tiny PoC:** Assemble tokeniser pipeline (likely tokenizers crate) and wire to onnxruntime. Validate header F1 on 20 sample PDFs.
- **Open an issue for Pipe-mode CLI:** Spec framing protocol; this is 90% async I/O plumbing, perfect for a community PR.

Let me know which of these resonates the most and I can flesh out deeper implementation scaffolding or spike code snippets. 