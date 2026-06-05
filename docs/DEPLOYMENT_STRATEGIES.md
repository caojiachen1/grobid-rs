# Deployment Strategies for grobid-rs

This document outlines various deployment strategies for grobid-rs, discussing the trade-offs between deployment modes, implementation approaches, and recommendations for different use cases.

## 1. Multiple Deployment Modes Overview

grobid-rs can be deployed in several ways, each optimized for different scenarios. Our architecture supports these modes through a consistent provider-based API design that gives users choice while maintaining clear boundaries between core and optional components.

### Core Design Philosophy

- Provide multiple deployment options without bloating the core library
- Maintain JNI as the fast, default path for highest performance
- Allow flexible integration patterns through feature-gated abstractions
- Enable consistent behavior across deployment modes

## 2. Deployment Modes Comparison

| Mode | Latency | Memory Usage | Best For | Key Characteristics |
|------|---------|--------------|----------|---------------------|
| **Direct JNI** (default) | 2-5ms | ~450MB | Performance-critical applications | Fastest execution after warm-up, single process, direct JVM integration |
| **Embedded Jetty** | +0.3-0.6ms | +15-30MB | HTTP-centric apps needing single binary | Small overhead, REST API in same process, single artifact distribution |
| **Side-car Service** | +1-5ms | 400-500MB (separate) | Microservice architecture, Kubernetes | Process isolation, crash resilience, horizontal scaling, polyglot compatibility |
| **Daemon Service** | 2-5ms | 500MB (persistent) | High-volume batch processing, server-side APIs | Eliminates JVM warm-up time (3-5s), persistent JVM, pre-loaded models |
| **Remote SaaS** | Network RTT | Zero local | Mobile/frontend apps, minimal footprint | No local resources, ideal for lightweight clients, network dependency |

*Latency figures are approximate based on benchmarks and user reports*

## 3. Architecture: Trait-Based Provider Abstraction

The core of our multi-mode deployment strategy is a trait-based abstraction:

```rust
/// Common interface for all Grobid provider implementations
pub trait GrobidProvider: Send + Sync {
    fn process_header(&self, pdf: &Path) -> Result<String, GrobidError>;
    fn process_references(&self, pdf: &Path) -> Result<String, GrobidError>;
    // ...other processing methods...
}
```

### Implementations

| Implementation | Cargo Feature | Description |
|----------------|--------------|-------------|
| `LocalEngine` | default | Current fast path using direct JNI calls |
| `RestClient` | rest-client | Thin wrapper using reqwest to call external Grobid REST API |
| `SidecarManager` | sidecar | Manages lifecycle of Docker or Java process with health checks |

Example usage:

```rust
// Default JNI implementation
let grobid = grobid_rs::LocalEngine::new(config)?;

// REST client implementation (with feature flag)
let grobid = grobid_rs::RestClient::new("http://localhost:8070")?;

// Code using the abstraction remains the same
let tei = grobid.process_header(Path::new("paper.pdf"))?;
```

## 4. Implementation Approaches for HTTP Layer

### Option A: Side-car Official Grobid Service

This approach runs the official Grobid Docker image or JAR alongside your Rust service.

**Implementation:**
1. Pull and run the Docker image: `grobid/grobid:0.9.1`
2. Connect to `http://localhost:8070/api/...` from your Rust application
3. Use a thin HTTP client instead of JNI for processing

**Pros:**
- Zero additional code in grobid-rs
- Complete compatibility with official Grobid REST API
- Process isolation (JVM crashes won't affect Rust application)

**Cons:**
- Two processes required
- Higher total memory usage (>900MB)
- More complex deployment and lifecycle management

### Option B: Embed Jetty Within JVM

This approach embeds the Jetty server within the same JVM that grobid-rs initializes.

**Implementation:**
1. Add Jetty dependencies to build configuration
2. Create a helper class to initialize the server:
   ```java
   Server server = new Server(8070);
   ServletContextHandler ctx = new ServletContextHandler(server, "/");
   ctx.addServlet(GrobidRestServlet.class, "/api/*");
   server.start();
   ```
3. Call this helper via JNI after ENGINE is initialized

**Pros:**
- Single artifact to distribute
- Reuses same Grobid engine objects
- Full compatibility with REST clients

**Cons:**
- Increased startup time (~500ms)
- Potential port collisions
- Additional JAR dependencies

### Option C: Implement REST Layer in Rust (Recommended)

This approach keeps JNI calls internal but exposes a Rust-native HTTP API that matches Grobid's endpoints.

**Implementation:**
1. Use existing JNI bindings to the engine
2. Create an HTTP server using Axum or Actix Web
3. Implement endpoints that match `/api/processHeaderDocument`, etc.
4. Return JSON or TEI XML using the same parameter handling as official Grobid

**Pros:**
- Full control over middleware (auth, tracing, metrics)
- No additional JVM dependencies
- Easier to embed in other Rust services
- Consistent with Rust's async ecosystem

**Cons:**
- Need to implement parameter handling logic to match official API
- Potential subtle differences in behavior

### Option D: Daemon Service

This approach runs grobid-rs as a long-running background service (daemon on Linux/macOS, Windows Service on Windows) to keep the JVM and Grobid models hot in memory.

**Implementation:**
1. Create a service wrapper around the existing JNI implementation
2. Expose an HTTP or Unix socket interface for communication
3. Provide service unit files for different platforms (systemd, launchd, NSSM)
4. Implement health checks and auto-restart capabilities

**Pros:**
- Eliminates cold-start overhead (3-8s for JVM startup and model loading)
- Enables horizontal scalability (multiple Rust workers share one Grobid instance)
- Provides language-agnostic access
- Allows system-level resource management (memory limits, GC tuning)

**Cons:**
- Persistent memory footprint (~500MB for resident JVM)
- Increased security surface (requires proper auth for exposed ports)
- Platform-specific service management

## 5. CLI Integration

The CLI will support all deployment modes:

```
# JNI (default)
grobid-cli header paper.pdf

# Connect to existing REST service
grobid-cli --remote http://srv:8070 header paper.pdf

# Start and manage Docker container
grobid-cli --docker-image grobid/grobid:0.9.1 header paper.pdf

# Run as a daemon service
grobid-cli daemon --listen :8070
```

Implementation simply builds the appropriate provider and passes it to library code. For daemon mode, the CLI would start a long-running service that keeps the JVM warm and exposes an HTTP API.

## 6. Implementation Plan

1. **Phase 1:** Refactor current API behind the `GrobidProvider` trait
   - Minimal changes to existing JNI implementation
   - Create trait definition and implement for current engine

2. **Phase 2:** Implement REST client
   - Add `rest-client` feature with reqwest dependency
   - Implement ~150 LOC wrapper for Grobid's REST API endpoints
   - Add tests that verify behavior matches JNI implementation

3. **Phase 3:** Create sidecar manager (optional)
   - Add `sidecar` feature
   - Implement process spawning and monitoring
   - Add health check and port forwarding

4. **Phase 4:** Feature flag integration
   - Update Cargo.toml with appropriate feature gates
   - Update CI matrix to build with all feature combinations

5. **Phase 5:** Documentation and examples
   - Add usage examples for each mode
   - Document performance characteristics
   - Provide guidance on choosing the right mode

6. **Phase 6:** Daemon mode implementation (optional)
   - Add `server` feature flag in Cargo.toml
   - Implement Axum/Actix Web service wrapper
   - Create service unit files for different platforms
   - Add daemon subcommand to CLI
   - Benchmark against batch workloads

## 7. Trade-offs and Considerations

### Risk Factors and Mitigations

| Risk | Mitigation |
|------|------------|
| Matrix explosion: every new Grobid version × 3 modes | Keep one Grobid version per crate release; run integration tests against all providers |
| Larger binary if Jetty included | Gate embedded-jetty behind a non-default feature and document the size impact (~20MB) |
| Memory footprint awareness | Emit warnings if available memory is below recommended thresholds |
| Security patch lag | Pin to LTS versions and regularly audit dependencies |
| Service management complexity | Provide templates and scripts for common platforms (systemd, launchd, NSSM) |
| Open ports in daemon mode | Document security best practices, default to localhost-only binding |

### Current Technical Details

- The current grobid-rs implementation uses JNI with an embedded JRE (produced via jlink in build.rs)
- No HTTP server is running in the default configuration
- Grobid's Java architecture has three layers:
  1. Low-level GrobidEngine
  2. Service façade (GrobidRestService)
  3. HTTP endpoints via Jetty

### Daemon Mode Decision Matrix

| Workload Type | Latency Benefits | Memory Considerations | Recommendation |
|---------------|------------------|------------------------|----------------|
| High-volume batch processing (100s+ PDFs/hour) | 2-3× faster (eliminating JVM warm-up) | Resident 1-2GB RAM acceptable | **YES** - daemon mode ideal |
| Server-side web app with per-request processing | Sub-second SLA requires resident JVM | Must sandbox untrusted PDFs | **YES** - with security controls |
| Occasional CLI use (dozens per day) | Startup cost negligible | Zero-install binaries preferred | **NO** - use embedded JNI |
| Edge/laptop tooling | Minimal benefit | RAM budget <1GB, battery concerns | **NO** - use on-demand JVM |

## 8. Recommendations

1. **Keep Direct JNI as Default**
   - It's the fastest path with lowest latency
   - Provides a single-binary distribution model
   - Simplest integration for Rust applications

2. **Implement Trait Abstraction Early**
   - Creates clean separation and future flexibility
   - Minimal changes to existing code

3. **Add REST Client Next**
   - Enables multi-language integration
   - Relatively simple implementation (~150 LOC)

4. **Document Trade-offs Clearly**
   - Help users make informed decisions
   - Highlight performance and resource implications

5. **Version Strategy**
   - Mark JNI as reference implementation with highest performance
   - Document REST/Docker as "good for language-agnostic setups but with performance trade-offs"

6. **Daemon Mode for High-Volume Processing**
   - Implement as an optional feature behind a flag (`--features server`)
   - Provide platform-specific service templates (systemd, launchd, NSSM)
   - Add auto-detection via environment variable (`GROBID_RS_ENDPOINT`)
   - Benchmark to quantify gains for batch processing

The multi-mode approach allows grobid-rs to serve a wide range of deployment scenarios while keeping the default implementation simple, performant, and maintainable.