# Model Modernisation: LayoutLM-tiny & OnnxRuntime for Grobid-RS

## Decision Memo: Proprietary Model Training with arXiv + Proprietary PDFs

This section covers:
1. **Licensing & IP Reality-Check**
2. **Practical Data-Engineering Steps**
3. **ROI & Resource Envelope**

---

## 1. Licensing & IP Reality-Check ⚖️

| Data source                | Licence / Terms         | You may …                                         | You must not / need …                                 | Bottom-line risk                |
|---------------------------|-------------------------|---------------------------------------------------|-------------------------------------------------------|-------------------------------|
| arXiv bulk PDFs           | Per-paper (CC-BY, etc.) | Text/data-mine for research; store model weights   | Redistribute PDFs unless licence allows; avoid No-licence/CC-BY-NC for commercial | Medium – curate white-list    |
| S2ORC (Semantic Scholar)  | CC-BY-NC 4.0            | Free research TDM                                 | No commercial reuse of raw text                        | Low for R&D; high for SaaS     |
| Grobid training TEI       | Apache-2.0              | Fully permissive                                  | —                                                     | None                          |
| Your own annotated PDFs   | Your choice             | Everything                                        | —                                                     | None                          |

**Are the weights proprietary?**
- Yes. Model weights are not a verbatim copy of the papers; you can license them as you wish, provided you have a lawful basis for the training data.

**Strategy to stay clean:**
1. White-list licences: keep only CC-BY, CC-BY-SA, CC0, or papers with explicit permission.
2. Store licence metadata in the training DB for auditability.
3. For No-licence papers: run inference only, never keep their raw text.

---

## 2. Practical "From-PDF-to-Model" Pipeline 🏗️

### Step 0 – Corpus curator 🔍
- Use arXiv/S2ORC APIs to filter by licence and build a JSON manifest.
- Download PDFs and store alongside `licence.json`.
- Time: 1–2 days for a Rust or Python downloader.

### Step 1 – Automatic TEI labelling 🏷️
- Use Grobid JVM to generate gold TEI for header/refs:
  ```sh
  grobid-cli header corpus/*.pdf -o tei/
  # Use -t 4 for parallelism
  ```

### Step 2 – Manual "spot-fix" loop 🔬
- Sample ~1,000 TEI files; crowdsource corrections (title, authors, DOI).
- Merge fixes back into training labels.
- 1K high-quality headers > 100K noisy ones.

### Step 3 – Feature / Token pipeline 🧩
- **CRF-compat:** Re-implement Grobid's feature generator in Rust; output tokens.tsv for Wapiti.
- **LayoutLM-tiny:** Use `tokenizers` crate + spatial embedding from bounding boxes (Poppler/pdfium). Output: `input_ids`, `bbox`, `attention_mask` tensors.
- Cost: ~1 week each; reuse a single PdfProvider trait.

### Step 4 – Training runs 🚂
| Model         | Framework         | GPU need | 1st run checklist                                  |
|---------------|-------------------|----------|----------------------------------------------------|
| CRF           | Wapiti C API/FFI  | CPU only | `wapiti train -t crf_l1` on ~500K tokens            |
| LayoutLM-tiny | HuggingFace/ONNX  | 1× GPU   | Freeze 4 layers, LR 1e-5, 3 epochs; expect F1 ↑ 5pt |

### Step 5 – Packaging 📦
- Store weights under `models/header_crf/` and `models/header_layoutlm/`.
- Embed a model registry JSON in your static bundle:
  ```json
  {
    "header": {
      "default": "layoutlm_tiny_v1",
      "fallback": "wapiti_2025_05"
    }
  }
  ```
- Add CLI flag: `--model header=crf` for A/B testing.

---

## 3. ROI & Resource Envelope 💰

| Item                        | Eng. time   | Cloud cost      | Impact                                      |
|-----------------------------|-------------|-----------------|---------------------------------------------|
| Downloader + Grobid labeller| 2 dev-days  | $0–$50 (bandwidth) | ~400 GB raw data for repeated experiments   |
| CRF re-train                | 3 days      | $50 CPU         | +1–2 pt F1; proof you can reproduce Grobid  |
| LayoutLM-tiny fine-tune     | 1 dev-week  | $200 GPU (~20h) | 5–10 pt F1 boost on multilingual headers    |
| Audit tooling & licence DB  | 2 days      | —               | De-risks commercial use; sell to "big-co"   |

---

## Actionable Next-Steps Checklist
- [ ] Create data-manifest repo (schema: `{pdf_url, sha256, licence, source, collected_at}`)
- [ ] Spin up pipeline infra (GitHub Action nightly job or Airflow DAG)
- [ ] Write a one-pager for management justifying GPU spend vs. F1 gain
- [ ] Start small: 10K PDFs, CRF re-train; use delta to baseline Grobid quality
- [ ] Parallel R&D branch for LayoutLM; reuse token pipeline code

---

## How to Keep Weights Proprietary and Compliant
- Use only data with clear, permissive licensing (CC-BY, CC0, your own docs)
- Keep licence metadata for every training sample
- Avoid GPL/AGPL code for model training/inference (MIT/Apache/BSD are safe)
- If using MIT-licensed base models (e.g., LayoutLMv3), retain attribution in your docs
- Ship GPL dependencies (e.g., Poppler) as separate dynamic libraries if needed

---

## Why This Matters
- **Legally viable:** Stick to CC-BY/CC0 slices of arXiv and your proprietary docs, keep licence metadata
- **Technically straightforward:** You already generate TEI—turn that into labels
- **Commercial upside:** Proprietary weights = defensible IP; "higher accuracy + JVM-free" is an easy sell

---

## Why It Matters

The current Grobid-RS pipeline relies on CRF models (Wapiti) for sequence labelling. While effective, CRFs are limited in their ability to capture complex, multilingual, and long-range document structure. Modern transformer models like LayoutLM-tiny, when run via ONNX and OnnxRuntime (ORT), can:
- Boost F1 by 5–10 points, especially on noisy or multilingual PDFs
- Enable single-pass inference (faster, lower latency)
- Remove JVM and JNI dependencies for ML
- Run in <100 MB RAM, suitable for serverless and edge
- Unlock new features (e.g., document classification, richer entity extraction)

## Technical Background

- **LayoutLM-tiny**: A transformer model pre-trained for document layout understanding. Accepts text, bounding boxes, and segment info.
- **ONNX**: Open Neural Network Exchange format; allows running models in Rust via the `onnxruntime` crate.
- **Tokenizers**: Use the `tokenizers` crate for fast, compatible tokenization.

## Implementation Plan

### 1. Model Preparation
- Select a pre-trained LayoutLM-tiny model (HuggingFace or custom-trained).
- Convert the model to ONNX format (using `transformers.onnx` or `optimum` tools).
- Validate ONNX export: ensure all ops are supported by `onnxruntime`.

### 2. Rust Integration
- Add `onnxruntime` and `tokenizers` as dependencies.
- Write a `layoutlm.rs` module (or crate) with:
  - Model loader (loads .onnx file, sets up ORT session)
  - Tokenizer loader (loads vocab, merges, etc.)
  - Inference function: accepts text, bboxes, returns tag sequence

```rust
use onnxruntime::{environment::Environment, session::Session, tensor::OrtOwnedTensor};
use tokenizers::Tokenizer;

pub struct LayoutLM {
    session: Session,
    tokenizer: Tokenizer,
}

impl LayoutLM {
    pub fn new(model_path: &str, tokenizer_path: &str) -> anyhow::Result<Self> {
        // Load ONNX model and tokenizer
    }
    pub fn predict(&self, text: &[String], bboxes: &[[i64; 4]]) -> anyhow::Result<Vec<String>> {
        // Tokenize, prepare tensors, run inference, decode tags
    }
}
```

### 3. Pipeline Integration
- Add a feature flag (e.g., `--ml-backend layoutlm` in CLI/config)
- Adapt the PDF → tokens → model → tags pipeline:
  - Extract text and bounding boxes from PDF (already in place)
  - Tokenize and align with LayoutLM input
  - Run inference, map output tags to entities
- Fallback to CRF for legacy/compatibility mode

### 4. Evaluation
- Validate on a sample set (e.g., 20 PDFs)
- Compare F1, precision, recall to CRF baseline
- Profile RAM and latency

### 5. Modularity & Extensibility
- Structure code so new models (e.g., LayoutLMv3, BERT) can be plugged in:
  - Define a `trait SequenceLabeler` with `predict()`
  - Implement for both CRF and LayoutLM
  - Allow dynamic selection at runtime

```rust
pub trait SequenceLabeler {
    fn predict(&self, text: &[String], bboxes: &[[i64; 4]]) -> anyhow::Result<Vec<String>>;
}

impl SequenceLabeler for LayoutLM { /* ... */ }
impl SequenceLabeler for CrfWapiti { /* ... */ }
```

### 6. Risks & Mitigations
- ONNX model export may require opset tweaks
- Tokenizer alignment (offsets, special tokens)
- Inference speed on ARM/low-end CPUs
- Model size (quantize if needed)

### 7. Future Directions
- Add support for full-text and references tasks
- Experiment with quantized or distilled models for WASM/edge
- Enable model hot-swapping via config
- Publish benchmarks and model zoo

## References
- [onnxruntime crate](https://crates.io/crates/onnxruntime)
- [tokenizers crate](https://crates.io/crates/tokenizers)
- [LayoutLM paper](https://arxiv.org/abs/1912.13318)
- [HuggingFace LayoutLM](https://huggingface.co/microsoft/layoutlm-base-uncased) 