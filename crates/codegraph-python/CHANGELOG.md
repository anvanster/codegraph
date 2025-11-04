# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-11-02

### Added

- Initial release of codegraph-python parser
- Parse single Python files with `parse_file()`
- Parse entire projects with `parse_project()`
- Parse source strings with `parse_source()`
- Extract function entities with parameters, return types, decorators
- Extract class entities with inheritance, methods, fields
- Extract module entities
- Track function call relationships
- Track import relationships
- Track inheritance relationships
- Track protocol/ABC implementation relationships
- Configurable parser behavior (visibility filtering, test inclusion, parallel processing)
- Comprehensive error handling without panics
- 90%+ code coverage
- Performance: Parse 1000 files in <10 seconds
- Support for Python 3.8+ syntax (async/await, type hints, decorators, match statements)

[Unreleased]: https://github.com/codegraph/codegraph-python/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/codegraph/codegraph-python/releases/tag/v0.1.0
