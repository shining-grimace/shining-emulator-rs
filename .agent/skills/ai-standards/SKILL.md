---
name: ai-standards
description: Coding standards for Bevy projects
---

# Rust & Bevy ECS Desktop App Architecture

*Defines the structural guidelines, idiomatic Rust practices, code organization, and ECS (Entity Component System) constraints for building scalable Rust desktop applications using the Bevy engine. These rules govern project structure, memory management, error handling, and system scheduling.*

## Context

*These rules apply to the entire Rust codebase. They are designed to leverage Rust's strict type system and memory safety, prevent spaghetti code, maximize Bevy's multi-threading capabilities, and ensure the app remains performant and maintainable as it scales into a large desktop application.*

**Applies to:** Cargo workspaces, module boundaries, Rust idioms, and all Bevy Plugins/Systems/Components.
**Level:** Strategic (Workspace/Module Architecture) and Operational (Code-level logic).
**Audience:** Developers and AI Coding Agents contributing to the application logic.

## Core Principles

1. **Make Invalid States Unrepresentable:** Leverage Rust's type system (Enums, Newtypes, State variables) to enforce correct logic at compile time. If a state shouldn't exist, the compiler shouldn't allow it to be written.
2. **Feature-First Organization:** Code must be organized by feature or domain (e.g., `settings`, `file_browser`, `rendering`), not by ECS type (e.g., not a global `systems` or `components` folder)
3. **Data-Oriented Design Over OOP:** Data (Components/Resources) and behavior (Systems/Traits) must remain strictly separated. Composition over inheritance is mandatory.
4. **Fearless Concurrency & Ownership:** Lean on Rust's borrow checker. Minimize allocations, avoid shared mutability (`Arc<Mutex<T>>`) when Bevy's `ResMut` or message passing (Events/Channels) can solve the problem natively.
5. **Single-responsibility Organization** Define common, shared parts together in files or modules which have single responsibility (a dimensions file, a colors file, a behavious directory, etc.) and try to keep feature files smaller than around 200 lines unless it's a large feature.

## Rules

### Must Have (Critical)
*Non-negotiable rules that must always be followed. Violation of these rules should block progress.*

- **RULE-001: No Panics in Production (`unwrap`/`expect`):** Never use `.unwrap()` or `.expect()` in application logic or systems. Use the `?` operator and handle errors gracefully. In Bevy systems, log errors or pipe them to an error-handling system rather than crashing the desktop app.
- **RULE-002: Feature-Based Module Structure:** Group related Systems, Components, and Events into domain-specific modules. A feature module (e.g., `src/ui/sidebar/`) should expose a `Plugin` and keep its internal systems private (`pub(crate)` or private).
- **RULE-003: No Unnecessary Exclusive Systems:** Never use `&mut World` in a system unless absolutely required (e.g., highly specialized low-level engine operations). Exclusive systems halt the multithreaded scheduler. Rely on `Commands`, `Query`, and `ResMut` instead.
- **RULE-004: Components Must Be Pure Data:** Components must be simple Rust `struct`s with no inherent business logic or associated `impl` blocks that mutate other application states. Logic belongs exclusively in Systems.

### Should Have (Important)
*Strong recommendations that should be followed unless there's a compelling reason not to.*

- **RULE-101: Semantic Error Handling:** Use `thiserror` for library/module-level error definitions to provide clear, typed error variants. Use `anyhow` only at the highest application boundaries or in binary entry points.
- **RULE-102: Leverage Change Detection:** Use Bevy's `Changed<T>` and `Added<T>` query filters to skip unnecessary work. Desktop apps often sit idle; do not recompute UI or state every frame if the underlying data hasn't mutated.
- **RULE-103: Avoid Defensive Cloning:** Do not use `.clone()` simply to appease the borrow checker. Rethink lifetimes, use borrowing (`&`, `&mut`), or use `Arc<T>` for heavy data payloads (like loaded files or large strings) that genuinely require shared ownership.
- **RULE-104: Workspace Separation:** For apps exceeding ~10,000 lines, split the project into Cargo Workspaces (e.g., `core_engine`, `ui_components`, `app_main`) to enforce strict dependency graphs and speed up incremental compilation.

### Could Have (Preferred)
*Best practices and preferences that improve quality but are not blocking.*

- **RULE-201: Standard Traits Over Custom Methods:** Implement standard Rust traits (`Default`, `Display`, `From`, `Into`) instead of writing custom `new()`, `to_string()`, or conversion methods where applicable.
- **RULE-202: Use Marker Components:** Use zero-sized components (e.g., `#[derive(Component)] struct IsSelected;`) as tags to easily query subsets of entities without adding memory overhead.
- **RULE-203: Newtype Pattern for IDs:** Wrap primitive types in single-element tuple structs (e.g., `struct WindowId(u32);`) to prevent accidentally swapping distinct IDs in function arguments.

## Patterns & Anti-Patterns

### ✅ Do This
*Organize by feature, handle errors without panicking, and use specific ECS queries.*

```rust
// File: src/file_system/mod.rs
use bevy::prelude::*;
use thiserror::Error;

// 1. Feature Plugin encapsulates domain logic
pub struct FileSystemPlugin;
impl Plugin for FileSystemPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, process_file_reads.map(handle_system_errors));
    }
}

// 2. Clear, typed errors
#[derive(Error, Debug)]
pub enum FileError {
    #[error("File not found: {0}")]
    NotFound(String),
}

// 3. Components are pure data (Newtype pattern)
#[derive(Component)]
pub struct FilePath(pub String);

#[derive(Component)]
pub struct LoadPending;

// 4. Systems are private/crate-level, use specific queries, and return Results
fn process_file_reads(
    mut commands: Commands,
    query: Query<(Entity, &FilePath), With<LoadPending>>
) -> Result<(), FileError> {
    for (entity, path) in query.iter() {
        // ... attempt load ...
        commands.entity(entity).remove::<LoadPending>();
    }
    Ok(())
}

// 5. Error pipeline system
fn handle_system_errors(In(result): In<Result<(), FileError>>) {
    if let Err(e) = result {
        eprintln!("System error: {}", e); // Or send to a UI error toast event
    }
}

❌ Don't Do This

Avoid flat organization, God structs, unwrap(), and optional blob fields.

// File: src/systems.rs (Anti-pattern: global systems file)
use bevy::prelude::*;

// Anti-pattern: God struct grouping unrelated domain data
#[derive(Component)]
pub struct GlobalAppData {
    pub file_path: Option<String>,
    pub window_title: String,
    pub is_loading: bool,
}

// Anti-pattern: Unwrapping and blind iteration
pub fn bad_update_files(mut query: Query<&mut GlobalAppData>) {
    for mut data in query.iter_mut() {
        if data.is_loading {
            // Anti-pattern: Panics if the path is somehow None
            let path = data.file_path.as_ref().unwrap(); 
            let _file = std::fs::read_to_string(path).unwrap(); // Blocks main thread AND panics
        }
    }
}

Decision Framework

When structuring a new module:

    Is it a standalone reusable piece of UI? Put it in src/ui/.

    Is it core business logic? Put it in src/domain_name/.

    If a module file exceeds ~500 lines, split it internally into components.rs, systems.rs, and events.rs within that domain's folder. Expose them via a mod.rs.

When system data access conflicts:

    Assess if both systems actually require mutable (&mut) access to the exact same component. Often, one system can be refactored to read-only (&).

    If both must mutate, check if they operate on mutually exclusive entities. If so, add With / Without filters to create disjoint queries.

When handling long-running tasks (e.g., Disk I/O, Network):

    Never block Bevy's main/update thread with synchronous I/O.

    Spaw asynchronous tasks using AsyncComputeTaskPool and communicate results back to the ECS via channels (e.g., crossbeam_channel) or by polling the task status in a Bevy system.

Exceptions & Waivers

Valid reasons for exceptions:

    Prototyping/Tests: Using .unwrap() is entirely acceptable and preferred inside test modules (#[cfg(test)]) to fail fast.

    FFI or Third-Party Desktop Integration: Integrating with synchronous desktop APIs (like native OS window handles) might require exclusive systems (&mut World) or bypass standard ECS rules.

Process for exceptions:

    Outline the exception in the module's inline documentation using /// # SAFETY or /// # EXCEPTIONS blocks.

    Isolate the breaking code into a dedicated, explicitly named Plugin (e.g., NativeDialogPlugin) to contain the architectural damage.

Quality Gates

    Automated checks: Run cargo clippy -- -D warnings -W clippy::pedantic. Enforce rustfmt formatting in CI pipelines.

    Code review focus: Reviewers must specifically look for .unwrap(), blocking I/O on the main thread, oversized components, and code leaking out of its domain folder.

    Testing requirements: Logic systems should be unit tested by creating a dummy World, adding the system via Schedule, running it, and asserting component state.

Related Rules

    rules/ui-layout-guidelines.md - Rules governing Bevy UI node hierarchies and styling conventions for desktop app layouts.

    rules/async-io.md - Guidelines for handling asynchronous tasks (network requests, database reads) within Bevy's synchronous ECS loop without blocking the frame rate.

References

    Rust API Guidelines - Standard Rust idioms and formatting.

    Bevy Official Documentation - The official Bevy Engine book.

    Bevy Cheatbook - Community-driven patterns, ECS recipes, and anti-patterns.

TL;DR

Key Principles:

    Organize code by feature/domain, not by ECS concept.

    Ensure maximum parallel execution by keeping data access minimal and handling I/O asynchronously.

    Leverage Rust's type system to make invalid application states unrepresentable.

Critical Rules:

    Never use .unwrap() or .expect() in production systems; handle errors via the type system.

    Never use &mut World (exclusive systems) unless fundamentally unavoidable.

    Keep components small and pure; never bundle unrelated data into a single god struct.
