# Windows 编译指南 — grobid-rs

> 默认开启所有 feature（cache / format / cli / parallel），普通 `cargo build` 即可。
> `--all-features` 与默认构建等价。

## 问题分析

当前 `vendor/jre/` 是 macOS 平台的 JRE（含 `.dylib` 文件），**Windows 无法使用**。 \
但你项目的其余部分对 Windows 的支持是完备的：

| 组件 | 位置 | Windows 支持？ |
|------|------|:---:|
| **Wapiti 原生库** | `vendor/grobid/grobid-home/lib/win-64/libwapiti.dll` | ✅ |
| **pdfalto 转换器** | `vendor/grobid/grobid-home/pdfalto/win-64/pdftoxml.exe` | ✅ |
| **Grobid JAR** | `vendor/grobid/grobid-core-0.9.1-onejar.jar.zst` | ✅ 跨平台 |
| **grobid-home (模型/配置)** | `vendor/grobid/grobid-home/` | ✅ 跨平台 |
| **裁剪 JRE (jlink)** | `vendor/jre/` 现有的是 macOS | ❌ 需在 Windows 上重建 |

---

## 前置条件

1. **Windows JDK 17+**（完整 JDK，不是 JRE），需要包含 `jlink.exe` 和 `jmods/` 目录
   - 下载：[Adoptium Temurin JDK 17](https://adoptium.net/temurin/releases/?version=17) 或 [Oracle JDK 17](https://www.oracle.com/java/technologies/javase/jdk17-archive-downloads.html)
   - 安装后设置环境变量：
     ```cmd
     set JAVA_HOME=C:\Program Files\Eclipse Adoptium\jdk-17.0.12.7-hotspot
     ```
   - 验证：
     ```cmd
     "%JAVA_HOME%\bin\javac" -version
     "%JAVA_HOME%\bin\jlink" --version
     ```

2. **Rust 工具链**（已安装）
   ```cmd
   rustup show
   ```

3. **zstd 命令行工具**（可选，用于加速解压）
   ```cmd
   # 用 chocolatey 或 scoop
   choco install zstd
   # 或
   scoop install zstd
   ```
   如果没有，`build.rs` 会自动用 Rust 库解压（慢一些但可用）。

---

## 编译步骤

### 方案 A：使用 vendor + 本地 jlink 重建 JRE（推荐）

`vendor/grobid/` 已有完整的 Grobid 资源，只需要在 Windows 上用你的 JDK 重新跑 `jlink` 生成 Windows 版 JRE。

```cmd
cd grobid-rs

REM 设置 JDK
set JAVA_HOME=C:\Program Files\Eclipse Adoptium\jdk-17.0.12.7-hotspot

REM 如果之前有 macOS 的构建产物，先清理掉
if exist target\grobid_assets\grobid-0.9.1\runtime rmdir /s /q target\grobid_assets\grobid-0.9.1\runtime

REM 编译（build.rs 会自动做三件事：
REM  1. 检测到 vendor 存在，跳过下载和 Gradle
REM  2. 复制 vendor/grobid/ 到 target/grobid_assets/
REM  3. 用 jlink 从你的 JDK 生成 Windows JRE 到 runtime/
REM  4. 配置 JNI 链接）
cargo build --release
```

**`build.rs` 在 Windows 上的行为**：

1. `check_for_vendored_files()` — 检测到 `vendor/grobid/` 和 `vendor/jre/` 存在 → 进入 vendor 路径
2. `use_vendored_files()` — 复制 `vendor/grobid/` 到部署目录 ✅
3. 复制 `vendor/jre/` 到部署目录 → **但是 vendor/jre/ 是 macOS 的** ⚠️
4. `ensure_jlink_runtime()` — 检测到 JRE 与当前环境不匹配（指纹不同）→ **自动用 jlink 重建 Windows JRE** ✅

所以核心问题只有一个：**vendor/jre/ 被 macOS 占位了，需要替换掉或确保 build.rs 识别到不匹配后重建**。

### 方案 B：直接从源码完整构建

如果不信任 vendor，也可以让 build.rs 下载 Grobid 源码 + 编译 + jlink 全流程。

```cmd
set JAVA_HOME=C:\Program Files\Eclipse Adoptium\jdk-17.0.12.7-hotspot
set FORCE_GROBID_REBUILD=true
cargo build --release
```

这会：
1. 从 GitHub 下载 `grobid-0.9.1.zip`（SHA-256 校验）
2. 用 `gradlew.bat` 编译 Grobid Java 代码
3. 用 `jlink` 生成 Windows JRE
4. 编译 Rust 代码（默认全 feature）

---

## 现有代码对 Windows 的支持情况

### ✅ 已正确处理的点

| 位置 | 代码 | 说明 |
|------|------|------|
| `build_modules/jre_ops.rs:12` | `if cfg!(windows) { "jlink.exe" } else { "jlink" }` | Windows 可执行文件后缀 |
| `build_modules/build_ops.rs` | `if cfg!(windows) { "gradlew.bat" } else { "gradlew" }` | Gradle wrapper 后缀 |
| `src/engine.rs:202` | `"windows" => "pdfalto.exe"` | pdfalto 可执行文件后缀 |
| `src/engine.rs:204` | `"windows" => "win-64"` | 平台目录 |
| `src/lib.rs:174-181` | `#[cfg(windows)] { PATH += "..." }` | Windows PATH 环境变量设置 |
| `src/lib.rs:130` | `lib_path = grobid_home.join("lib/win-64")` | Windows 原生库路径 |
| `build.rs` | `decompress_zstd_file()` 内 Rust fallback | Windows 上没有 zstd CLI 时自动降级 |
| `build_modules/jni_config.rs:39` | `if cfg!(windows) { "bin/server" } else { "lib/server" }` | JVM DLL 目录 |
| `build_modules/jni_config.rs:72` | 根据 OS 选择链接方式 | Windows 用 `jvm.lib` import library |
| `src/lib.rs:291` | `"wapiti.dll"` / `"libwapiti.dll"` 自动检测 | Windows DLL 名称处理 |

### ⚠️ 需要注意的点

1. **路径中的反斜杠**
   `src/lib.rs` 已经用 `.replace('\\', "/")` 统一转为 JVM 能识别的正斜杠 ✅

2. **`vendor/jre/` 是 macOS 的，需要替换**
   最简单的方法：**删除 `vendor/jre/`**，让 `build.rs` 的 `ensure_jlink_runtime()` 从你的 JDK 重建。
   或者更好：提交一个占位文件 `.gitkeep`，把 macOS JRE 从 git 中移除。

3. **`src/lib.rs` 的 JAR 路径硬编码问题**

   当前 `lib.rs:153-155`：
   ```rust
   let grobid_core = grobid_path.join("grobid-core");
   let grobid_core_jar = grobid_core.join("build/libs/grobid-core-0.9.1-onejar.jar");
   ```

   这个路径只存在于 Gradle 编译输出结构中。如果用 vendor 路径，JAR 实际在 `target/grobid_assets/grobid-0.9.1/grobid-core-0.9.1-onejar.jar`。

   但看你的 `target/grobid_assets/grobid-0.9.1/` 目录，里面的 `grobid/grobid-core/build/` 确实存在（之前 Gradle 构建产物保留着），所以这个路径**目前工作正常**。

   但如果有人从干净的 vendor 构建（没有 Gradle 输出），就会报错找不到 JAR。需要改成用编译时 `env!("GROBID_JAR_PATH")` 来获取路径。

4. **Windows 长路径**
   `target\grobid_assets\grobid-0.9.1\...` 嵌套很深，可能超过 Windows 260 字符路径限制。
   如需缩短，设置输出目录：
   ```cmd
   set GROBID_RS_ASSETS_PATH=C:\tmp\grobid-assets
   ```

---

## 完整 Windows 编译脚本

创建 `build_windows.cmd`：

```batch
@echo off
chcp 65001 >nul

echo =======================================
echo   grobid-rs Windows Build
echo =======================================

REM 配置 JDK 路径（按你的实际路径修改）
set JAVA_HOME=C:\Program Files\Eclipse Adoptium\jdk-17.0.12.7-hotspot

REM 验证 JDK
if not exist "%JAVA_HOME%\bin\jlink.exe" (
    echo [ERROR] JDK not found at %JAVA_HOME%
    echo Set JAVA_HOME to a JDK 17+ installation
    exit /b 1
)
echo [OK] JDK: %JAVA_HOME%

REM 清理旧的 JRE 缓存（平台不匹配时）
if exist target\grobid_assets\grobid-0.9.1\runtime (
    echo [INFO] Removing cached runtime for fresh build...
    rmdir /s /q target\grobid_assets\grobid-0.9.1\runtime
)

REM 编译（默认开启所有 feature）
echo [INFO] Building...
cargo build --release

if %ERRORLEVEL% EQU 0 (
    echo [OK] Build successful!
    echo Binary: target\release\grobid-cli.exe
    echo Features enabled: cache, format, cli, parallel
) else (
    echo [ERROR] Build failed
    exit /b %ERRORLEVEL%
)
```

---

## 验证

编译成功后：

```cmd
REM 查看 CLI 帮助
target\release\grobid-cli --help

REM 处理 PDF 标题（测试 PDF 已经有了）
target\release\grobid-cli header test_pdfs\SynCode-LLM-Generation-with-Grammar-Augmentation.pdf

REM 输出 JSON
target\release\grobid-cli header test_pdfs\SynCode-LLM-Generation-with-Grammar-Augmentation.pdf --output-format json

REM 参考文献提取
target\release\grobid-cli references test_pdfs\SynCode-LLM-Generation-with-Grammar-Augmentation.pdf

REM 并行批量处理（parallel feature）
target\release\grobid-cli batch --parallel test_pdfs\*.pdf --output output_dir
```

---

## 遇到问题怎么办

### "Runtime directory not found"

`build.rs` 的 `ensure_jlink_runtime()` 没有成功生成 JRE。

排查：
```cmd
REM 检查部署目录
dir target\grobid_assets\grobid-0.9.1\ /B

REM 检查有没有 runtime 目录
dir target\grobid_assets\grobid-0.9.1\runtime
```

如果没有 `runtime/`，说明 `jlink` 执行失败。手动试试：
```cmd
"%JAVA_HOME%\bin\jlink" --module-path "%JAVA_HOME%\jmods" ^
    --add-modules java.base,java.logging,java.xml,jdk.unsupported,java.naming,java.desktop,java.sql,java.management ^
    --strip-debug --no-header-files --no-man-pages --compress=2 ^
    --output target\grobid_assets\grobid-0.9.1\runtime
```

### "java.library.path" 找不到 wapiti

原生库路径问题。确认：
```cmd
dir vendor\grobid\grobid-home\lib\win-64\
```
应该能看到 `libwapiti.dll`。

### "no jlink in JDK" / "jmods not found"

你的 JAVA_HOME 可能指向的是 JRE 而不是 JDK。JRE 没有 `jlink` 和 `jmods`，必须安装完整 JDK。

验证：
```cmd
dir "%JAVA_HOME%\bin\jlink.exe"
dir "%JAVA_HOME%\jmods"
```
两者都必须存在。

---

## Feature 说明

| Feature | 默认启用 | 作用 |
|---------|:--------:|------|
| `cache` | ✅ | PDF 内容缓存（SHA-256 哈希键），避免重复处理 |
| `format` | ✅ | TEI→JSON / Text / BibTeX 格式转换 |
| `cli` | ✅ | `grobid-cli` 命令行工具 + 进度条 + 日志 |
| `parallel` | ✅ | 基于 `rayon` 的多线程并行批量处理 |

手动选择性编译：
```cmd
REM 仅缓存 + 格式转换（库模式，无 CLI）
cargo build --release --no-default-features --features cache,format

REM 最小库（仅引擎，无转换、无缓存）
cargo build --release --no-default-features
```

---

## 总结

| 步骤 | 需要你做 | 已自动完成 |
|------|----------|-----------|
| 安装 Windows JDK 17+ | ✅ 下载 + 设 `JAVA_HOME` | — |
| Grobid 资源（JAR + 模型 + Wapiti DLL） | ❌ 不需要 | `vendor/grobid/` 已有 |
| Windows JRE | ❌ 不需要手动 | `build.rs` 在编译时用 jlink 生成 |
| Rust 代码 | ❌ 不需要改 | 已全平台支持 |
| pdfalto | ❌ 不需要 | `vendor/grobid/` 已有 Windows 版 |

**一句话**：装好 Windows JDK，设好 `JAVA_HOME`，直接 `cargo build --release` 就能编出来，默认即全部 feature。
