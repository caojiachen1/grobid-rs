# License Mining Proof-of-Concept for grobid-rs

## 1 · Why License-Mining is Valuable

| Angle                | Pay-off                                                                                 |
|----------------------|----------------------------------------------------------------------------------------|
| Compliance automation| Vendors must ship license BOMs (L-BOM) for SBOM/CRA/FedRAMP; automates a manual task.  |
| Due-diligence / M&A  | Buyers can scan repos & PDFs for license conflicts (GPL in proprietary, NC in ML, etc.) |
| Academic publishing  | Journals/funders want machine-readable license statements in deposited PDFs.            |
| Competitive moat     | Grobid already extracts structure; license detection is a natural, hard-to-copy add-on. |

---

## 2 · Legal/IP Feasibility

| Data Source         | Licence / Terms         | You may …                                 | You must not / need …                        | Bottom-line risk |
|--------------------|------------------------|-------------------------------------------|----------------------------------------------|------------------|
| SPDX licence texts | CC-0                   | Use, redistribute, train freely           | —                                           | None             |
| GitHub repos       | MIT/BSD/Apache, GH ToS | Use public license files for training      | Attribute if required                        | None             |
| arXiv papers       | Author copyright, CC-0 | Use metadata, mine embedded license blurbs | Don't redistribute PDFs unless allowed       | Low              |
| Annotation         | Factual                | Label spans as license text               | Don't embed large verbatim text in weights   | None             |
| Model weights      | Your copyright         | Keep proprietary, license as you wish      | Don't embed verbatim text above fair use     | None             |

- **Summary:** You can train and ship a proprietary model as long as you use permissive data, keep annotation factual, and avoid embedding large verbatim text in weights.

---

## 3 · High-Level Architecture

```
                ┌────────────┐                    ┌──────────────┐
PDF / Markdown ─►  Text+Box   ├─► Rule heuristics ─► Candidate    │
 & source code  └──────┬──────┘                    │  passages    │
                       │                          /└──────────────┘
                       │                         /
                       ▼                        /       Fine-tuned
                 LayoutLM-tiny  <───────────────         model
                (or any Transformer)             (token / span CLS)
```

- **Step 1:** Pre-filter with regexes/heuristics ("license", SPDX ids, ©, etc.) to extract candidate spans.
- **Step 2:** LayoutLM-tiny (or similar) classifies each candidate as license/disclaimer/other.
- **Step 3:** Post-process into SBOM-style JSON: `{package, file, license, confidence, offsets}`.

---

## 4 · Step-by-Step PoC Roadmap

### 4.1 Data Set
| Bucket                | How to collect                                         | Size target |
|-----------------------|--------------------------------------------------------|-------------|
| Clean licence texts   | SPDX list + GitHub license templates                   | 1–2k docs   |
| PDF licences          | arXiv API (license=CC*), fetch PDFs                    | 5–10k docs  |
| Messy embedded        | Sample repos with mixed licensing via GH search        | 1–2k files  |

- **Annotation tip:** Use weak-labelling: auto-label exact SPDX matches, manually review uncertain spans.

### 4.2 Baseline Heuristic
- 20–30 regexes + Levenshtein on SPDX ids
- Expect recall ≈ 0.85 / precision ≈ 0.70
- Log false positives for ML training

### 4.3 LayoutLM-tiny Fine-tune (text + bbox)
- Tokenizer: `tokenizers` crate (WordPiece/BPE)
- Model: HuggingFace LayoutLMv3-base → prune/distil to "tiny"
- Framework: `onnxruntime` crate (portable, no CUDA needed)
- Training: tangram or tch-rs; log to W&B/TensorBoard
- Eval: F1 on span-level (IoU ≥ 0.5 = hit). Target F1 > 0.93

### 4.4 Integration into grobid-rs
- Expose `LicenseDetector` trait (heuristic vs. ML)
- CLI: `grobid-cli licenses file.pdf` → JSON output for SBOM
- HTTP: `/api/processLicenses` endpoint

---

## 5 · ROI & Risk Table

| Lens             | Upside                                                      | Caveat                                         |
|------------------|-------------------------------------------------------------|------------------------------------------------|
| Short-term PoC   | Shows compliance/novelty in <1 month                        | Needs benchmarks vs. scancode-toolkit/Licensee |
| Commercial IP    | You own model weights; SaaS or on-prem possible             | Keep training data/pipeline private            |
| SBOM alignment   | Auto-fills CycloneDX/SPDX package.licenses                  | Legal teams want explainability (highlighting) |
| Extensible       | Can mine ethics, data-usage, funder mandates                | Annotation cost rises with more label types    |

---

## 6 · 1-Week Action Plan
- [ ] Kick-off `license-miner-rs` repo in workspace
- [ ] Write collector script for SPDX + arXiv (Rust/Python, <200 LOC)
- [ ] Implement heuristic-only detector: `LicenseDetector::heuristic()`; wire to CLI `--licenses-heuristic`
- [ ] Report metrics on sample set; log errors
- [ ] Open tracking issues for LayoutLM fine-tune and SBOM JSON schema

---

## 7 · Bottom Line

License mining is feasible, leverages your existing PDF & TEI pipeline, and has clear commercial pull (SBOM, audits, due-diligence). Start with a regex-plus-metadata baseline this week; parallel-track LayoutLM-tiny fine-tune; ship a feature-flagged CLI command as early tech preview and iterate from there. 