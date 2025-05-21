# Licensing Information for grobid-rs

This document details the licensing terms for grobid-rs and all bundled components.

## grobid-rs Licensing

The grobid-rs Rust library is dual-licensed under:
- MIT License
- Apache License, Version 2.0

You may choose either license at your option.

## Third-Party Components

grobid-rs bundles or interacts with several third-party components, each with its own license:

### Grobid

- **Component**: Grobid core library
- **License**: Apache License 2.0
- **Source**: https://github.com/kermitt2/grobid
- **Requirements**: Include Apache 2.0 license text and any NOTICE files when redistributing

### Wapiti JNI Library

- **Component**: Wapiti machine learning library with JNI bindings 
- **License**: BSD 3-Clause License
- **Source**: Included with Grobid
- **Requirements**: Include license notice in redistributions

### pdfalto

- **Component**: Tool for converting PDF to ALTO XML
- **License**: GNU General Public License v3.0 (GPL-3.0)
- **Source**: https://github.com/kermitt2/pdfalto
- **Requirements**: 
  - When distributing pdfalto binaries, you must also provide the corresponding source code or a written offer to provide the source code.
  - Include a copy of the GPL-3.0 license.
  - Note that pdfalto is invoked as a separate executable, which is considered "mere aggregation" and does not require grobid-rs itself to be licensed under GPL-3.0.

### OpenJDK Components

- **Component**: Custom Java Runtime Environment (JRE) created via jlink
- **License**: GNU General Public License, version 2, with the Classpath Exception (GPL-2.0 with CPE)
- **Source**: OpenJDK
- **Requirements**:
  - Include the GPL-2.0 license text.
  - Include notice about the Classpath Exception.
  - Provide access to the OpenJDK source code when distributing the bundled JRE.

### Rust Dependencies

The Rust components of grobid-rs depend on various crates, primarily licensed under:
- MIT License
- Apache License 2.0

A complete list of dependencies and their licenses can be generated using:
```
cargo license
```

## Compliance Checklist

When distributing grobid-rs:

1. Include the complete text of all relevant licenses (MIT, Apache 2.0, GPL-3.0, GPL-2.0 with CPE, BSD) in a `licenses` directory.
2. Maintain the existing copyright and license notices in all source files.
3. Include a copy of this LICENSING.md file.
4. For GPL components (pdfalto and OpenJDK):
   - Provide source code or written offer for source code.
   - Ensure these components are invoked as separate processes and not statically linked.
5. Create an attribution notice that lists all components and their respective licenses.

## Attribution

The following attribution notice should be included in your documentation:

```
This application includes:

- grobid-rs (MIT/Apache 2.0)
- Grobid (Apache 2.0)
- Wapiti (BSD 3-Clause)
- pdfalto (GPL-3.0)
- OpenJDK components (GPL-2.0 with Classpath Exception)
- Various Rust libraries (primarily MIT/Apache 2.0)

Full licensing information and source code availability information 
can be found in the LICENSING.md file.
```

## Legal Notice

This licensing information is provided for guidance only and does not constitute legal advice. Consult with a legal professional regarding your specific use case and distribution requirements.