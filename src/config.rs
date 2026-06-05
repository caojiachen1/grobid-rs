use crate::GrobidError;
use crate::LogLevel;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Configuration for the Grobid engine.
#[derive(Debug, Clone)]
pub struct GrobidConfig {
    /// Path to the directory containing the Grobid deployment files.
    pub base_path: PathBuf,

    /// Maximum memory allocation for the JVM (-Xmx option).
    pub max_memory: String,

    /// Additional JVM options.
    pub jvm_options: Vec<String>,

    /// Number of concurrent threads for processing (when using parallel processing).
    pub thread_count: usize,

    /// Custom Java system properties.
    pub system_properties: HashMap<String, String>,

    /// Verbosity level for logging.
    pub log_level: LogLevel,

    /// Whether to prefer vendored installation if available.
    pub prefer_vendored: bool,

    /// Analysis configuration for Grobid processing.
    pub analysis_config: Option<GrobidAnalysisConfig>,
}

/// Configuration for Grobid document analysis.
#[derive(Debug, Clone)]
pub struct GrobidAnalysisConfig {
    /// Whether to consolidate header metadata with external services
    pub consolidate_header: bool,

    /// Whether to consolidate citations with external services
    pub consolidate_citations: bool,

    /// Whether to include coordinates for text blocks
    pub include_coordinates: bool,

    /// Whether to segment sentences in the output
    pub segment_sentences: bool,

    /// Whether to generate raw citations
    pub generate_raw_citations: bool,
}

impl Default for GrobidAnalysisConfig {
    fn default() -> Self {
        Self {
            consolidate_header: false,
            consolidate_citations: false,
            include_coordinates: false,
            segment_sentences: false,
            generate_raw_citations: true,
        }
    }
}

impl Default for GrobidConfig {
    fn default() -> Self {
        Self {
            base_path: PathBuf::from(env!("GROBID_RS_ASSETS_PATH")),
            max_memory: "1G".to_string(),
            jvm_options: Vec::new(),
            thread_count: 1,
            system_properties: HashMap::new(),
            log_level: LogLevel::Info,
            prefer_vendored: false,
            analysis_config: None,
        }
    }
}

impl GrobidConfig {
    /// Create a new configuration with the given base path.
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            ..Default::default()
        }
    }

    /// Set the maximum memory allocation for the JVM.
    pub fn with_max_memory(mut self, max_memory: impl Into<String>) -> Self {
        self.max_memory = max_memory.into();
        self
    }

    /// Add a JVM option.
    pub fn with_jvm_option(mut self, option: impl Into<String>) -> Self {
        self.jvm_options.push(option.into());
        self
    }

    /// Set the number of threads to use for parallel processing.
    pub fn with_thread_count(mut self, thread_count: usize) -> Self {
        self.thread_count = thread_count;
        self
    }

    /// Add a system property for the JVM.
    pub fn with_system_property(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.system_properties.insert(key.into(), value.into());
        self
    }

    /// Set the log level.
    pub fn with_log_level(mut self, log_level: LogLevel) -> Self {
        self.log_level = log_level;
        self
    }

    /// Set whether to prefer vendored files.
    pub fn with_prefer_vendored(mut self, prefer_vendored: bool) -> Self {
        self.prefer_vendored = prefer_vendored;
        self
    }

    /// Set the analysis configuration.
    pub fn with_analysis_config(mut self, config: GrobidAnalysisConfig) -> Self {
        self.analysis_config = Some(config);
        self
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), GrobidError> {
        if !self.base_path.exists() {
            return Err(GrobidError::Configuration(format!(
                "Base path does not exist: {}",
                self.base_path.display()
            )));
        }

        if !self.base_path.is_dir() {
            return Err(GrobidError::Configuration(format!(
                "Base path is not a directory: {}",
                self.base_path.display()
            )));
        }

        // Check if the thread count is sensible
        if self.thread_count == 0 {
            return Err(GrobidError::Configuration(
                "Thread count must be greater than zero".to_string(),
            ));
        }

        // Check Grobid version compatibility
        let properties_path = self
            .base_path
            .join("grobid-home/config/grobid.properties");
        if properties_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&properties_path) {
                // Look for version line (grobid.version=X.Y.Z)
                if let Some(line) = content.lines().find(|l| l.starts_with("grobid.version=")) {
                    if let Some(found_version) = line.strip_prefix("grobid.version=") {
                        // Compare with expected version
                        let expected_version = "0.9.1"; // Hardcoded for now, should be a constant
                        if found_version != expected_version {
                            return Err(GrobidError::Configuration(format!(
                                "Grobid version mismatch: expected {}, found {}",
                                expected_version, found_version
                            )));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Create a builder for the configuration.
    pub fn builder() -> GrobidConfigBuilder {
        GrobidConfigBuilder::default()
    }
}

/// Builder for creating GrobidConfig instances.
#[derive(Debug, Default)]
pub struct GrobidConfigBuilder {
    base_path: Option<PathBuf>,
    max_memory: Option<String>,
    jvm_options: Vec<String>,
    thread_count: Option<usize>,
    system_properties: HashMap<String, String>,
    log_level: Option<LogLevel>,
    prefer_vendored: Option<bool>,
    analysis_config: Option<GrobidAnalysisConfig>,
}

impl GrobidConfigBuilder {
    /// Set the base path for Grobid resources.
    pub fn base_path(mut self, path: impl AsRef<Path>) -> Self {
        self.base_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Set the maximum memory allocation for the JVM.
    pub fn max_memory(mut self, memory: impl Into<String>) -> Self {
        self.max_memory = Some(memory.into());
        self
    }

    /// Add a JVM option.
    pub fn jvm_option(mut self, option: impl Into<String>) -> Self {
        self.jvm_options.push(option.into());
        self
    }

    /// Set the number of threads for parallel processing.
    pub fn thread_count(mut self, count: usize) -> Self {
        self.thread_count = Some(count);
        self
    }

    /// Add a system property.
    pub fn system_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.system_properties.insert(key.into(), value.into());
        self
    }

    /// Set the log level.
    pub fn log_level(mut self, level: LogLevel) -> Self {
        self.log_level = Some(level);
        self
    }

    /// Set whether to prefer vendored files.
    pub fn prefer_vendored(mut self, prefer: bool) -> Self {
        self.prefer_vendored = Some(prefer);
        self
    }

    /// Configure Grobid analysis options.
    pub fn analysis_config(self) -> GrobidAnalysisConfigBuilder {
        GrobidAnalysisConfigBuilder {
            parent_builder: self,
            config: GrobidAnalysisConfig::default(),
        }
    }

    /// Build the final GrobidConfig.
    pub fn build(self) -> GrobidConfig {
        let base_path = self
            .base_path
            .unwrap_or_else(|| PathBuf::from(env!("GROBID_RS_ASSETS_PATH")));

        GrobidConfig {
            base_path,
            max_memory: self.max_memory.unwrap_or_else(|| "1G".to_string()),
            jvm_options: self.jvm_options,
            thread_count: self.thread_count.unwrap_or(1),
            system_properties: self.system_properties,
            log_level: self.log_level.unwrap_or(LogLevel::Info),
            prefer_vendored: self.prefer_vendored.unwrap_or(false),
            analysis_config: self.analysis_config,
        }
    }
}

/// Builder for GrobidAnalysisConfig.
pub struct GrobidAnalysisConfigBuilder {
    parent_builder: GrobidConfigBuilder,
    config: GrobidAnalysisConfig,
}

impl GrobidAnalysisConfigBuilder {
    /// Configure whether to consolidate header metadata.
    pub fn consolidate_header(mut self, value: bool) -> Self {
        self.config.consolidate_header = value;
        self
    }

    /// Configure whether to consolidate citations.
    pub fn consolidate_citations(mut self, value: bool) -> Self {
        self.config.consolidate_citations = value;
        self
    }

    /// Configure whether to include coordinates.
    pub fn include_coordinates(mut self, value: bool) -> Self {
        self.config.include_coordinates = value;
        self
    }

    /// Configure whether to segment sentences.
    pub fn segment_sentences(mut self, value: bool) -> Self {
        self.config.segment_sentences = value;
        self
    }

    /// Configure whether to generate raw citations.
    pub fn generate_raw_citations(mut self, value: bool) -> Self {
        self.config.generate_raw_citations = value;
        self
    }

    /// Finish configuring analysis options and return to the parent builder.
    pub fn done(self) -> GrobidConfigBuilder {
        GrobidConfigBuilder {
            analysis_config: Some(self.config),
            ..self.parent_builder
        }
    }
}
