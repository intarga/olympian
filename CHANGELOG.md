# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- SpatialCache and SeriesCache structs, to simplify the signatures of QC tests.
- Documentation for all public items

### Changed

- All QC test signatures have been changed to use SpatialCache/SeriesCache, for the timeseries tests, this also means that they have been adapted to handle QCing multiple values by windowing, instead of leaving that to the caller.
- Fixed a bug in dip_check where `Flag::Fail` would never be returned, all `Flag::Fail`s would instead be `Flag::Warn`.

### Removed

- SpatialTree has been removed from the public API, it should be used through SpatialCache instead.

[unreleased]: https://github.com/intarga/olympian
