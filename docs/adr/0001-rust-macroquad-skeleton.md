# ADR 0001: Rust Macroquad Skeleton

## Status

Accepted

## Context

Skylands needs a starting architecture for the First Playable Version: a deterministic simulation core, a true 3D City Screen skeleton, versioned Save State, and enough rendering/input wiring to prove later Building and Road features.

The project targets native desktop first while keeping the architecture portable to web later. It should avoid full engine bloat.

## Decision

- Use a Cargo workspace with `skylands_core` and `skylands_app`.
- Keep `skylands_core` free of rendering dependencies.
- Model deterministic simulation with plain structs plus typed commands.
- Start with commands for `StartRun`, `Tick`, and placeholder `PlaceBuilding`.
- Store world tiles as integer tile coordinates plus integer height.
- Generate one deterministic irregular Flying Island from a seed.
- Place a fixed City Core on a valid generated footprint.
- Use `macroquad` for the first true 3D app skeleton.
- Use a fixed isometric camera with pan and zoom, leaving the camera API extensible for later rotation.
- Let the app derive render view data from read-only `RunState`.
- Define explicit versioned save structs in `skylands_core` and serialize them as JSON.
- Start with Rust unit tests for simulation/domain behavior.

## Rejected Alternatives

- Browser-first TypeScript/Vite/Three.js: rejected because the project direction is Rust and native desktop first.
- `wgpu` directly: rejected for now because the skeleton needs momentum more than low-level rendering control.
- A full engine such as Unity or Godot: rejected to avoid engine bloat.
- ECS-style simulation: rejected until the domain shows enough entity/system complexity to justify it.
- Mixed logic/render game objects: rejected because Simulation Tick, placement, SkyCoin, Food, Citizens, Performance, Bonuses, and Save State need deterministic testing.
- Serializing the entire live simulation as the public save contract: rejected in favor of deliberate `SaveStateV1` structs.

## Consequences

The first runnable app proves the City Screen architecture with simple colored 3D cuboids, a deterministic Flying Island, tile hover highlighting, and a HUD showing tick-linked run values.

Later work can add Road rules, Building costs, richer economy formulas, free camera rotation, asset loading, and platform-specific save adapters without changing the core boundary.
