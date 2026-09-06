# grobid-rs 使用指南

## 目录

- [安装](#安装)
- [初始化](#初始化)
- [API 速查表](#api-速查表)
- [详细 API 说明](#详细-api-说明)
  - [初始化与生命周期](#1-初始化与生命周期)
  - [低层引擎 API（TEI XML）](#2-低层引擎-api-tei-xml)
  - [高层 API](#3-高层-api)
  - [缓存系统](#4-缓存系统)
  - [数据模型](#5-数据模型)
  - [错误处理](#6-错误处理)
- [完整示例](#完整示例)

---

## 安装

`Cargo.toml`：

```toml
[dependencies]
grobid-rs = { git = "https://github.com/caojiachen1/grobid-rs" }

# 如果只需要缓存功能（默认即开启）：
grobid-rs = { git = "https://github.com/caojiachen1/grobid-rs" }

# 按需选择 features（以下为当前默认值，不写也行）：
grobid-rs = { git = "https://github.com/caojiachen1/grobid-rs", features = ["cache", "format", "cli", "parallel"] }
```

| Feature | 默认 | 作用 |
|---------|:----:|------|
| `cache`  | ✅ | 基于 SHA-256 的 PDF 缓存，避免重复处理 |
| `format` | ✅ | TEI XML → JSON / Text / BibTeX 格式转换 |
| `cli`    | ✅ | CLI 二进制 `grobid-cli` |
| `parallel` | ✅ | 基于 `rayon` 的多线程批量处理 |

### 系统要求

| 条件 | 说明 |
|------|------|
| Rust 1.65+ | 稳定版即可 |
| JDK 11+ | **仅构建时需要**，用于 `jlink` 生成 JRE。设 `JAVA_HOME` |
| 磁盘空间 | 构建产物约 1.3 GB（模型 + JRE + JAR） |
| 内存 | 处理 PDF 最少 1 GB RAM |

---

## 初始化

`grobid-rs` 在使用前必须调用 `init()`。JVM 和 Grobid 引擎会以单例方式启动，每个进程只需一次。

```rust
use std::path::Path;

// === 方式一：默认配置（自动检测构建产物路径） ===
let config = grobid_rs::GrobidConfig::builder().build();
grobid_rs::init(&config)?;

// === 方式二：手动指定资源路径（推荐用于打包/分发） ===
let config = grobid_rs::GrobidConfig::builder()
    .base_path("/path/to/grobid-assets")  // 含 JAR + grobid-home/ + runtime/
    .max_memory("2G")
    .thread_count(4)
    .log_level(grobid_rs::LogLevel::Debug)
    .system_property("my.key", "my.value")
    .jvm_option("-XX:+UseG1GC")
    .analysis_config()                    // 进入分析配置构建器
        .consolidate_header(true)
        .consolidate_citations(true)
        .include_coordinates(false)
        .segment_sentences(false)
        .generate_raw_citations(true)
        .done()                           // 返回主构建器
    .build();

grobid_rs::init(&config)?;

// === 方式三：极简（new + 链式调用） ===
grobid_rs::init(
    &grobid_rs::GrobidConfig::new("/path/to/grobid-assets")
        .with_max_memory("4G")
        .with_thread_count(8)
        .with_log_level(grobid_rs::LogLevel::Info)
)?;
```

### 生命周期函数

```rust
// 初始化（幂等，多次调用安全）
grobid_rs::init(&config)?;

// 检查是否已初始化
if grobid_rs::is_initialized() {
    println!("Grobid 已就绪");
}

// 关闭（清理引擎引用）
grobid_rs::shutdown()?;
```

---

## API 速查表

| 处理类型 | TEI XML（原始） | JSON | 结构化 Rust 类型 |
|---------|----------------|------|----------------|
| **全文** | `grobid_rs::fulltext_to_tei()` | `grobid_rs::fulltext_to_json()` | `grobid_rs::fulltext_to_structured()` → `GrobidDocument` |
| **头部** | `grobid_rs::process_header()` | `grobid_rs::process_header_json()` | `grobid_rs::process_header_structured()` → `DocumentMetadata` |
| **参考文献** | `grobid_rs::process_references()` | `grobid_rs::process_references_json()` | `grobid_rs::process_references_structured()` → `Vec<Reference>` |
| **自定义** | — | — | `grobid_rs::parse_tei_str()` → `ParsedTei`（解析任意 TEI） |

---

## 详细 API 说明

### 1. 初始化与生命周期

#### `GrobidConfig`

```rust
pub struct GrobidConfig {
    pub base_path: PathBuf,          // 资源目录（含 JAR + grobid-home/ + runtime/）
    pub max_memory: String,          // JVM 最大堆内存 "-Xmx"，默认 "1G"
    pub jvm_options: Vec<String>,    // 额外 JVM 参数
    pub thread_count: usize,         // 并行线程数，默认 1
    pub system_properties: HashMap<String, String>,  // 自定义 -D 系统属性
    pub log_level: LogLevel,         // 日志级别，默认 Info
    pub prefer_vendored: bool,       // 是否优先使用集成文件
    pub analysis_config: Option<GrobidAnalysisConfig>,  // 分析配置
}
```

构造方法：

```rust
// GrobidConfig::new(base_path) — 快速创建
let config = grobid_rs::GrobidConfig::new("/path/to/assets");

// GrobidConfig::builder() — 完整构建器
let config = grobid_rs::GrobidConfig::builder()
    .base_path("/path/to/assets")
    .max_memory("2G")
    .thread_count(4)
    .log_level(grobid_rs::LogLevel::Debug)
    .jvm_option("-XX:+UseG1GC")
    .system_property("key", "val")
    .prefer_vendored(true)
    .build();

// 链式修改（new 之后）
let config = grobid_rs::GrobidConfig::new("/path")
    .with_max_memory("4G")
    .with_thread_count(8)
    .with_log_level(grobid_rs::LogLevel::Trace)
    .with_jvm_option("-XX:+UseZGC")
    .with_system_property("key", "val")
    .with_prefer_vendored(true)
    .with_analysis_config(my_analysis_config);
```

#### `GrobidAnalysisConfig`

```rust
pub struct GrobidAnalysisConfig {
    pub consolidate_header: bool,       // 是否与外部服务合并头部，默认 false
    pub consolidate_citations: bool,    // 是否与外部服务合并引用，默认 false
    pub include_coordinates: bool,      // 是否包含坐标信息，默认 false
    pub segment_sentences: bool,        // 是否分割句子，默认 false
    pub generate_raw_citations: bool,   // 是否生成原始引用，默认 true
}
```

使用构建器：

```rust
let analysis = grobid_rs::GrobidAnalysisConfig::builder()
    .consolidate_header(true)
    .consolidate_citations(true)
    .include_coordinates(false)
    .segment_sentences(true)
    .generate_raw_citations(true)
    .done()  // ← 注意返回 GrobidConfigBuilder，不是最终 Config
    .build(); // ← 最终构建 GrobidConfig
```

#### `LogLevel`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}
```

#### 常量

```rust
pub const GROBID_VERSION: &str = "0.9.1";
```

---

### 2. 低层引擎 API（TEI XML）

这三个函数返回 Grobid 原生 TEI XML（字符串），是最底层的 API。

#### `grobid_rs::process_header`

```rust
pub fn process_header(pdf: &Path) -> Result<String, GrobidError>
```

输入：PDF 文件路径
输出：TEI XML 字符串（头部元数据：标题、作者、摘要等）

```rust
let tei = grobid_rs::process_header(Path::new("paper.pdf"))?;
println!("{}", tei); // ← TEI XML
```

#### `grobid_rs::fulltext_to_tei`

```rust
pub fn fulltext_to_tei(pdf: &Path) -> Result<String, GrobidError>
```

输入：PDF 文件路径
输出：TEI XML 字符串（全文 + 头部 + 参考文献）

```rust
let tei = grobid_rs::fulltext_to_tei(Path::new("paper.pdf"))?;
```

#### `grobid_rs::process_references`

```rust
pub fn process_references(pdf: &Path) -> Result<String, GrobidError>
```

输入：PDF 文件路径
输出：TEI XML 字符串（仅参考文献列表）

```rust
let tei = grobid_rs::process_references(Path::new("paper.pdf"))?;
```

#### `grobid_rs::run_pdfalto`

```rust
pub fn run_pdfalto(pdf: &Path, grobid_home: &Path) -> Result<PathBuf, GrobidError>
```

运行 pdfalto（PDF → ALTO XML 工具），返回生成的 ALTO XML 文件路径。

```rust
let alto_path = grobid_rs::run_pdfalto(
    Path::new("paper.pdf"),
    Path::new("/path/to/grobid-home"),
)?;
// 生成 paper.alto.xml
```

---

### 3. 高层 API

每个处理类型提供三种输出格式：

| 后缀 | 用途 |
|------|------|
| `_json()` | 返回 JSON 字符串 |
| `_json_with_options(pdf_path, pretty)` | 返回 JSON，控制是否美化 |
| `_structured()` | 返回强类型 Rust 结构体 |

#### 头部 API

```rust
// 头部 JSON（美化）
let json = grobid_rs::process_header_json(Path::new("paper.pdf"))?;

// 头部 JSON（紧凑）
let json = grobid_rs::process_header_json_with_options(
    Path::new("paper.pdf"),
    false,  // pretty = false → 紧凑
)?;

// 头部结构化
let metadata: grobid_rs::DocumentMetadata =
    grobid_rs::process_header_structured(Path::new("paper.pdf"))?;

println!("标题: {:?}", metadata.title);
println!("作者数: {}", metadata.authors.len());
println!("摘要: {:?}", metadata.abstract_text);
```

#### 全文 API

```rust
// 全文 JSON（美化）
let json = grobid_rs::fulltext_to_json(Path::new("paper.pdf"))?;

// 全文 JSON（紧凑）
let json = grobid_rs::fulltext_to_json_with_options(
    Path::new("paper.pdf"),
    false,
)?;

// 全文结构化
let doc: grobid_rs::GrobidDocument =
    grobid_rs::fulltext_to_structured(Path::new("paper.pdf"))?;

println!("来源: {}", doc.source);
println!("版本: {}", doc.version);
println!("参考文献数: {}", doc.references.len());

if let Some(full_text) = &doc.full_text {
    println!("章节数: {}", full_text.sections.len());
    println!("图表数: {}", full_text.figures.len());
    println!("表格数: {}", full_text.tables.len());
}
```

#### 参考文献 API

```rust
// 参考文献 JSON（美化）
let json = grobid_rs::process_references_json(Path::new("paper.pdf"))?;

// 参考文献 JSON（紧凑）
let json = grobid_rs::process_references_json_with_options(
    Path::new("paper.pdf"),
    false,
)?;

// 参考文献结构化
let refs: Vec<grobid_rs::Reference> =
    grobid_rs::process_references_structured(Path::new("paper.pdf"))?;

for r in &refs {
    println!("  [{}] {:?}", r.id.as_deref().unwrap_or("?"), r.title.as_deref().unwrap_or("?"));
    println!("    作者: {}", r.authors.join("; "));
    println!("    DOI: {:?}", r.doi);
}
```

#### `parse_tei_str` — 解析任意 TEI XML

```rust
pub fn parse_tei_str(tei: &str) -> Result<ParsedTei, GrobidError>
```

```rust
let tei_xml = grobid_rs::fulltext_to_tei(Path::new("paper.pdf"))?;

match grobid_rs::parse_tei_str(&tei_xml)? {
    grobid_rs::ParsedTei::Header(metadata) => {
        println!("仅头部: {:?}", metadata.title);
    }
    grobid_rs::ParsedTei::References(refs) => {
        println!("仅引用: {} 条", refs.len());
    }
    grobid_rs::ParsedTei::Full(doc) => {
        println!("完整文档: {} 条引用", doc.references.len());
    }
}
```

---

### 4. 缓存系统

#### `CacheConfig`

```rust
pub struct CacheConfig {
    pub enabled: bool,           // 是否启用缓存，默认 true
    pub skip_existing: bool,     // 命中缓存是否跳过处理，默认 true
    pub force_reprocess: bool,   // 是否强制重新处理，默认 false
}
```

#### `OutputType`

```rust
pub enum OutputType { Tei, Json, Bibtex, Text }

impl OutputType {
    pub fn extension(&self) -> &'static str {
        // Tei → "tei", Json → "json", Bibtex → "bib", Text → "txt"
    }
}
```

#### `CacheStats`

```rust
pub struct CacheStats {
    pub hits: usize,           // 缓存命中次数
    pub misses: usize,         // 缓存未命中次数
    pub bytes_read: usize,     // 从缓存读取的字节数
    pub bytes_written: usize,  // 写入缓存的字节数
    pub time_saved_ms: u64,    // 缓存估计节省的时间（毫秒）
}
```

#### 缓存处理 API

```rust
// 通用缓存处理
let result = grobid_rs::process_with_cache(
    Path::new("paper.pdf"),
    grobid_rs::OutputType::Json,  // 缓存输出类型
    grobid_rs::CacheConfig {
        enabled: true,
        skip_existing: true,
        force_reprocess: false,
    },
    || grobid_rs::fulltext_to_tei(Path::new("paper.pdf")),  // 处理函数
)?;

// 全功能快捷函数
let tei = grobid_rs::fulltext_to_tei_cached(
    Path::new("paper.pdf"),
    grobid_rs::CacheConfig::default(),
)?;

let tei = grobid_rs::process_header_cached(
    Path::new("paper.pdf"),
    grobid_rs::CacheConfig::default(),
)?;

let tei = grobid_rs::process_references_cached(
    Path::new("paper.pdf"),
    grobid_rs::CacheConfig::default(),
)?;
```

#### 缓存底层操作

```rust
// 获取缓存目录
let cache_dir = grobid_rs::get_cache_dir()?;

// 获取某个 PDF + 输出类型的缓存路径
let cache_path = grobid_rs::get_cache_path(Path::new("paper.pdf"), grobid_rs::OutputType::Json)?;

// 获取缓存的路径（如果存在）
let path = grobid_rs::get_cached_path(Path::new("paper.pdf"), grobid_rs::OutputType::Tei)?;

// 检查缓存是否存在
let exists = grobid_rs::cache_exists(Path::new("paper.pdf"), grobid_rs::OutputType::Tei)?;

// 读取缓存内容
let cached = grobid_rs::read_cache(Path::new("paper.pdf"), grobid_rs::OutputType::Tei)?;

// 写入缓存
grobid_rs::write_cache(Path::new("paper.pdf"), grobid_rs::OutputType::Tei, "TEI 内容")?;

// 获取缓存统计
let stats = grobid_rs::get_cache_stats();
println!("命中: {}, 未命中: {}, 节省: {}ms",
    stats.hits, stats.misses, stats.time_saved_ms);

// 重置缓存统计
grobid_rs::reset_cache_stats();
```

#### 缓存管理

```rust
// 裁剪缓存（限制最大大小）
let (removed_files, removed_bytes) = grobid_rs::prune_cache(1024 * 1024 * 500)?;  // 限制 500 MB
println!("清理了 {} 个文件，释放了 {} 字节", removed_files, removed_bytes);

// 清空所有缓存
let (removed, bytes) = grobid_rs::clear_cache()?;

// 获取缓存大小
let size_bytes = grobid_rs::get_cache_size()?;

// 获取人类可读的缓存大小
let size_str = grobid_rs::get_human_readable_cache_size()?;
println!("缓存大小: {}", size_str);

// 列出所有缓存文件
let files = grobid_rs::list_cache_files()?;

// 缓存摘要
let summary = grobid_rs::get_cache_summary()?;
println!("{}", summary);

// 启动后台 GC 线程（每小时检查并裁剪）
grobid_rs::start_background_gc();

// 手动触发检查和裁剪
grobid_rs::check_and_prune_if_needed()?;

// 确保缓存目录存在
grobid_rs::ensure_cache_dir()?;
```

**环境变量：**

| 变量 | 作用 | 默认 |
|------|------|------|
| `GROBID_RS_CACHE_DIR` | 覆盖缓存目录 | 系统默认缓存目录 |
| `GROBID_RS_CACHE_MAX_SIZE` | 最大缓存大小（字节） | 10 GB |
| `GROBID_RS_CACHE_AUTO_PRUNE` | 启用自动裁剪 | 取决于实现 |

---

### 5. 数据模型

#### `GrobidDocument`

```rust
pub struct GrobidDocument {
    pub source: String,                          // 始终 "grobid-rs"
    pub version: String,                         // 库版本（来自 Cargo.toml）
    pub metadata: DocumentMetadata,              // 文档元数据
    pub full_text: Option<FullText>,             // 全文（可选，仅 fulltext 模式）
    pub references: Vec<Reference>,              // 参考文献
}
```

所有字段自动 `Serialize` + `Deserialize`，可直接序列化为 JSON。

#### `DocumentMetadata`

```rust
pub struct DocumentMetadata {
    pub title: Option<String>,                   // 文档标题
    pub authors: Vec<Author>,                    // 作者列表
    pub abstract_text: Option<String>,           // 摘要
    pub date: Option<Date>,                      // 出版日期
    pub doi: Option<String>,                     // DOI
    pub venue: Option<Venue>,                    // 发表期刊/会议
    pub keywords: Vec<String>,                   // 关键词
    pub other: HashMap<String, String>,          // 其他元数据
}
```

#### `Author`

```rust
pub struct Author {
    pub first_name: Option<String>,
    pub middle_name: Option<String>,
    pub last_name: Option<String>,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub affiliation: Option<String>,             // 机构
    pub identifier: Option<String>,              // ORCID 等
}
```

#### `Date`

```rust
pub struct Date {
    pub year: Option<String>,
    pub month: Option<String>,
    pub day: Option<String>,
    pub raw: Option<String>,                     // 原始日期字符串
}
```

#### `Venue`

```rust
pub struct Venue {
    pub name: Option<String>,                    // 期刊/会议名称
    pub volume: Option<String>,                  // 卷
    pub issue: Option<String>,                   // 期
    pub pages: Option<String>,                   // 页码范围
    pub publisher: Option<String>,
}
```

#### `FullText`

```rust
pub struct FullText {
    pub sections: Vec<Section>,                  // 各章节
    pub figures: Vec<Figure>,                    // 图片
    pub tables: Vec<Table>,                      // 表格
    pub equations: Vec<Equation>,                // 公式
}
```

#### `Section`

```rust
pub struct Section {
    pub title: Option<String>,
    pub level: u8,                                // 层级（1 = 一级标题）
    pub content: String,                          // 正文
    pub subsections: Vec<Section>,                // 子章节
}
```

#### `Figure` / `Table` / `Equation`

```rust
pub struct Figure {
    pub id: Option<String>,
    pub caption: Option<String>,
    pub description: Option<String>,
}

pub struct Table {
    pub id: Option<String>,
    pub caption: Option<String>,
    pub content: Option<String>,
}

pub struct Equation {
    pub id: Option<String>,
    pub content: String,                          // MathML 或 LaTeX
    pub description: Option<String>,
}
```

#### `Reference`

```rust
pub struct Reference {
    pub id: Option<String>,
    pub title: Option<String>,
    pub authors: Vec<String>,                     // 作者名（扁平字符串列表）
    pub date: Option<Date>,
    pub venue: Option<String>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub pages: Option<String>,
    pub publisher: Option<String>,
    pub doi: Option<String>,
    pub raw: Option<String>,                      // 原始引用文本
}
```

#### `ParsedTei`

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ParsedTei {
    Header(DocumentMetadata),       // 仅头部
    References(Vec<Reference>),     // 仅参考文献
    Full(GrobidDocument),          // 完整文档
}
```

---

### 6. 错误处理

所有 fallible 函数返回 `Result<_, GrobidError>`。

#### `GrobidError` 变体

```rust
pub enum GrobidError {
    NotInitialised,                // init() 未调用
    Jni(JniError),                 // JNI 交互错误
    JvmInitialization(String),     // JVM 启动失败
    Java(String),                  // Java 端异常
    PdfAlto(String),               // pdfalto 工具错误
    InvalidInput(String),          // 输入参数无效（如路径不存在）
    Configuration(String),         // 配置错误
    Io(std::io::Error),            // 文件 I/O 错误
    VersionMismatch { expected, found },  // Grobid 版本不匹配
    Cache(String),                 // 缓存错误
    ParseError(String),            // XML 解析错误
    UnexpectedEof(String),         // 文件意外结束
    XmlParseError { message, context },  // XML 解析上下文错误
    MalformedXml { message, expected, found },  // 格式错误
    SerializationError(String),    // JSON 序列化错误
    DeserializationError(String),  // JSON 反序列化错误
    Conversion(String),            // 类型转换错误
}
```

#### 便捷构造方法

```rust
// File not found
GrobidError::file_not_found("path/to/file.pdf");

// Invalid input
GrobidError::invalid_input("file is empty");

// Version mismatch
GrobidError::version_mismatch("0.9.1", "0.8.2");
```

#### 典型错误处理模式

```rust
use grobid_rs::GrobidError;

fn process_pdf(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::Path::new(path);

    // 匹配特定错误
    match grobid_rs::process_header(path) {
        Ok(tei) => println!("成功: {}", tei),
        Err(GrobidError::NotInitialised) => {
            eprintln!("请先调用 init()");
        }
        Err(GrobidError::InvalidInput(msg)) => {
            eprintln!("输入无效: {}", msg);
        }
        Err(e) => {
            eprintln!("处理失败: {}", e);
        }
    }

    // 或者用 ? 传播错误
    let metadata = grobid_rs::process_header_structured(path)?;
    Ok(())
}
```

---

## 完整示例

### 基础：提取头部 + 全文 + 参考文献

```rust
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 初始化
    grobid_rs::init(&grobid_rs::GrobidConfig::builder().build())?;

    let pdf = Path::new("paper.pdf");

    // 2. 提取头部（结构化）
    let meta = grobid_rs::process_header_structured(pdf)?;
    println!("标题: {:?}", meta.title);
    println!("作者: {}", meta.authors.iter()
        .filter_map(|a| a.full_name.as_deref())
        .collect::<Vec<_>>()
        .join(", "));
    println!("摘要: {:?}", meta.abstract_text.map(|s| s.chars().take(200).collect::<String>()));

    // 3. 提取全文（JSON 输出）
    let json = grobid_rs::fulltext_to_json(pdf)?;
    std::fs::write("paper.json", json)?;

    // 4. 提取参考文献
    let refs = grobid_rs::process_references_structured(pdf)?;
    for (i, r) in refs.iter().enumerate() {
        println!("[{}] {} — {}", i + 1,
            r.title.as_deref().unwrap_or("(无标题)"),
            r.authors.join("; "));
    }

    Ok(())
}
```

### 带缓存的生产用例

```rust
use std::path::Path;
use grobid_rs::{CacheConfig, process_with_cache, OutputType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    grobid_rs::init(&grobid_rs::GrobidConfig::builder().build())?;

    let pdf = Path::new("paper.pdf");

    let result = process_with_cache(
        pdf,
        OutputType::Json,
        CacheConfig {
            enabled: true,
            skip_existing: true,    // 已有缓存 → 跳过处理
            force_reprocess: false,  // 不强制重处理
        },
        || grobid_rs::fulltext_to_json(pdf),
    )?;

    println!("{}", result);

    // 查看缓存统计
    let stats = grobid_rs::get_cache_stats();
    println!("缓存: {} 命中 / {} 未命中，节省 {}ms",
        stats.hits, stats.misses, stats.time_saved_ms);

    Ok(())
}
```

### 自定义配置

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = grobid_rs::GrobidConfig::builder()
        .max_memory("4G")
        .thread_count(8)
        .log_level(grobid_rs::LogLevel::Info)
        .analysis_config()
            .consolidate_header(true)
            .consolidate_citations(true)
            .include_coordinates(false)
            .segment_sentences(true)
            .done()
        .build();

    grobid_rs::init(&config)?;

    let meta = grobid_rs::process_header_structured(Path::new("paper.pdf"))?;

    // 序列化为 JSON
    println!("{}", serde_json::to_string_pretty(&meta)?);

    Ok(())
}
```

### 缓存管理

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 启动后台自动裁剪（每小时检查）
    grobid_rs::start_background_gc();

    // 或手动裁剪
    let (removed, freed) = grobid_rs::prune_cache(500 * 1024 * 1024)?; // 限制 500MB
    println!("清理了 {} 个文件，释放 {} MB", removed, freed / 1024 / 1024);

    // 查看缓存摘要
    println!("{}", grobid_rs::get_cache_summary()?);

    // 清空缓存
    grobid_rs::clear_cache()?;

    Ok(())
}
```

### 批量处理多个 PDF

```rust
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = grobid_rs::GrobidConfig::builder()
        .thread_count(4)
        .build();

    grobid_rs::init(&config)?;

    let pdfs = vec!["doc1.pdf", "doc2.pdf", "doc3.pdf"];

    // 带缓存的批量处理
    for pdf_name in &pdfs {
        let pdf = Path::new(pdf_name);
        match grobid_rs::process_header_cached(pdf, grobid_rs::CacheConfig::default()) {
            Ok(tei) => println!("{} → 成功 ({} bytes)", pdf_name, tei.len()),
            Err(e) => eprintln!("{} → 失败: {}", pdf_name, e),
        }
    }

    Ok(())
}
```

---

## 在 Tauri 中集成

参考 [Tauri 资源打包](https://v2.tauri.app/develop/resources/)：

```json
// tauri.conf.json
{
  "bundle": {
    "resources": ["grobid-assets/**"]
  }
}
```

```rust
// Rust 端初始化
let resource_dir = app.path().resource_dir()?;
let grobid_base = resource_dir.join("grobid-assets");

let config = grobid_rs::GrobidConfig::builder()
    .base_path(&grobid_base)
    .build();

grobid_rs::init(&config)?;
```

或者首次启动时下载到用户数据目录：

```rust
let app_data = app.path().app_data_dir()?;
let grobid_dir = app_data.join("grobid-assets");

if !grobid_dir.exists() {
    // 下载和解压逻辑
    download_and_extract("https://releases.example.com/grobid-assets.tar.zst", &grobid_dir)?;
}

let config = grobid_rs::GrobidConfig::builder()
    .base_path(&grobid_dir)
    .build();

grobid_rs::init(&config)?;
```

---

> 完整示例代码参见仓库下的 [`examples/`](examples/) 目录。
