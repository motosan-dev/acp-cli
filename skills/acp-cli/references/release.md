# Release Process

## Tag Convention

| Tag format | Registry | Workflow |
|------------|----------|----------|
| `vX.Y.Z` | crates.io | `publish.yml` |

## Release Checklist

### 1. Bump version

File: `Cargo.toml` → `version = "X.Y.Z"`

### 2. Update CHANGELOG

File: `CHANGELOG.md`

```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
- ...

### Changed
- ...

### Fixed
- ...
```

### 3. Commit

```bash
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release vX.Y.Z"
```

### 4. Tag + Push

```bash
git tag -a vX.Y.Z -m "vX.Y.Z — summary of changes"
git push origin main vX.Y.Z
```

This triggers `publish.yml`:
1. `cargo fmt --check`
2. `cargo clippy -- -D warnings`
3. `cargo test`
4. `cargo publish` (secret: `CARGO_REGISTRY_TOKEN`)

## GitHub Secrets

| Secret | Used by | Purpose |
|--------|---------|---------|
| `CARGO_REGISTRY_TOKEN` | publish.yml | Authenticate to crates.io |

## CI Workflows

| Workflow | Trigger | Steps |
|----------|---------|-------|
| `ci.yml` | Push/PR | fmt → clippy → test |
| `publish.yml` | Tag `v*` or manual | fmt → clippy → test → publish |

## Emergency Manual Publish

```bash
cargo publish
```
