# Making Grobid-RS' PDF Layer Pluggable

You're already halfway there: everything above the "give me text + coordinates" step is your domain-specific logic; everything below that is "just" a PDF renderer / parser. Treat it like any other I/O provider and you unlock a lot of flexibility:

## Why Bother?

| Reason         | Typical Examples |
|---------------|-----------------|
| **Performance** | `pdfium-render` can rasterise & extract text on GPU; poppler is CPU-bound but has zero royalties; MuPDF is tiny and great for serverless. |
| **Licensing / deployment** | Some clients will accept AGPL (poppler), others need BSD/MIT. Being able to swap back-ends keeps your license Apache / MIT. |
| **Special features** | OCR fallback, table extraction, embedded font recovery, page images for thumbnails. |
| **Future proofing** | Native Rust PDF ecosystem (e.g. pdf-writer, printpdf, tener-pdf) is moving quickly; you can adopt when ready without rewriting Grobid logic. |

---

## Proposed Architecture

```
grobid-rs
├─ core/                – JNI engine, TEI → Rust, JSON, caching
├─ pdf/
│  ├─ mod.rs            – trait + feature-gated re-exports
│  ├─ poppler.rs        – FFI wrapper (default)
│  ├─ pdfium.rs         – calls pdfium-render
│  ├─ mupdf.rs          – optional, via C API
│  └─ pure_rust.rs      – placeholder for future `pdf` crate
└─ cli/                 – uses pdf::* through the trait
```

### 1. Define a Minimal Façade Trait

```rust
/// What Grobid actually needs, expressed in pure Rust.
pub trait PdfProvider: Send + Sync {
    /// Parse one page, return UTF-8 text blocks + bbox (points).
    fn extract_page(&self, page_index: u32) -> anyhow::Result<Vec<TextBlock>>;

    /// Optional: rasterise page for figure detection.
    fn raster_page(&self, page_index: u32, dpi: u16) -> Option<image::RgbaImage> { None }

    /// Optional: plain bytes if caller wants to hand the file to JVM untouched.
    fn raw(&self) -> &[u8];
}
// Automatically implemented for &T, Arc<T>, etc.
```

### 2. Gate Concrete Back-ends Behind Cargo Features

```toml
[features]
default = ["poppler"]
poppler  = ["poppler-sys"]
pdfium   = ["pdfium-render"]
mupdf    = ["mupdf-sys"]
pure     = ["pdf"]          # early adopter

[target.'cfg(windows)'.dependencies]
pdfium-render = { version = "0.8", optional = true }
```

The CLI just adds clap flags:

```
--pdf-backend {poppler|pdfium|mupdf}
--dpi 150                 # passed only if the chosen backend supports raster
```

### 3. Compile-time Selection First, Dynamic Loading Later
- Compile-time is trivial: `#[cfg(feature="pdfium")]` in `pdf::pdfium`.
- Dynamic (user-supplied .so/.dll) is possible via libloading, but you can postpone – start simple.

### 4. Try a Reference Implementation

Pick the one you know best (pdfium-render is ergonomic):

```rust
pub struct PdfiumProvider {
    doc: pdfium_render::prelude::PdfDocument,
    bytes: Arc<Vec<u8>>,
}
impl PdfProvider for PdfiumProvider {
    fn extract_page(&self, idx: u32) -> Result<Vec<TextBlock>> {
        let page = self.doc.page_by_index(idx)?;
        Ok(page
            .objects()
            .iter()
            .filter_map(|o| o.as_text())
            .flat_map(|t| t.text_lines().iter())
            .map(|l| TextBlock {
                text: l.text().to_string(),
                bbox: l.bounding_box(),
            })
            .collect())
    }
    fn raster_page(&self, idx: u32, dpi: u16) -> Option<image::RgbaImage> {
        self.doc.page_by_index(idx).ok()?.render_with_dpi(dpi).ok()
    }
    fn raw(&self) -> &[u8] { &self.bytes }
}
```

### 5. Wire it into Grobid-RS
- Wherever you currently pass a Path to the JVM, accept `impl PdfProvider`.
- For the "JNI path", if the provider can't give you raw bytes, dump to a temp file – still cheaper than re-parsing.

---

## Migration Plan

| Sprint | Deliverable |
|--------|-------------|
| 0 | Move existing poppler code behind the PdfProvider trait – no functional change. |
| 1 | Add pdfium feature (Windows/macOS first, Linux via official binaries). |
| 2 | CLI flag + config file support; emit a warning if the requested back-end wasn't compiled in. |
| 3 | Benchmark (time, memory) on a 100-PDF corpus; document results in docs/pdf_backends.md. |
| 4 | Optional: expose the trait as a plugin API (cdylib + C ABI): lets third parties add e.g. OCR-heavy back-ends without rebuilding grobid-rs. |

---

## Where pymupdf fits

Great for quick prototypes or data science notebooks.
You could ship a Python plugin that talks to Grobid-RS via a tiny gRPC or JSON-RPC shim – but that feels outside an MVP for now.

---

## TL;DR – Yes, Start Now
- The trait (+ feature flags) is only ~150 LOC.
- You'll unblock experiments (pdfium, MuPDF, pure Rust).
- Users keep one mental model: "Grobid-RS speaks PDF, pick your engine."

Let me know which back-end you'd like to prototype first (pdfium is usually the smoothest). 