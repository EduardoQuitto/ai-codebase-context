# ADR 0006 — Provisional binary name `aicc`

- Status: accepted (provisional, reversible)
- Date: 2026-10-06

## Context

ROADMAP leaves the final product/binary name open. The foundation needs *something* stable to compile, test and document without baking in a brand.

## Decision

Use internal working name `aicc` (crate `aicc-*`, binary `aicc`, cache dir `aicc`). All user-visible strings derive from `aicc-core::branding` constants. Final naming validated later against GitHub / crates.io / package-manager collisions, pronunciation and searchability.

## Consequences

- Rename = change `branding.rs` + package metadata + docs; no logic depends on the string.
- Never reference the legacy `Enviro` name; it belongs to a different layer (environment vs context).
