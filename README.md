# YaST3 Prototype: Service Module Implementation

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-prototype-yellow.svg)]()

> **A safety-first system configuration engine with predictable, reversible operations**

This prototype implements the **Service Module** of YaST3 (Yet Another Setup Tool 3), demonstrating the core safety model and operational workflow outlined in the full system design.

## 📋 Overview

YaST3 is a next-generation system configuration engine that treats change as a controlled process rather than a sequence of commands. This prototype focuses on service management to validate the fundamental architecture:

- **Explicit State Management**: Clear separation between desired and observed states
- **Safe Execution**: Dry-run simulation before any system modification
- **Automatic Rollback**: First-class rollback operations planned alongside execution
- **Structured Reporting**: Human and machine-readable outcomes, not just logs

## 🎯 Problem Statement

Modern system configuration tools execute changes with limited visibility and poor recovery mechanisms. Common issues include:

- No preview of planned changes before execution
- Partial failures leaving systems in inconsistent states
- Manual or absent rollback procedures
- Difficulty diagnosing failures across different tools

YaST3 addresses these by making safety, simulation, and recovery integral to every operation.

## 🔬 Research Context

This prototype serves as a proof-of-concept for the safety model described in the [System Design Overview](docs/System_Design_Overview.pdf). It demonstrates:

1. **Predictable Change Management**: Declarative intent → inspection → planning → simulation → execution
2. **Failure-Aware Design**: Rollback plans generated before any state modification
3. **Clean Abstraction**: Core engine separated from system-specific implementations
4. **Observable Outcomes**: Structured reports enabling auditing and automation

## ⚙️ Implemented Features

### Core Workflow
- ✅ **Inspect**: Compare desired state against current system state
- ✅ **Plan**: Generate ordered action lists with dependency resolution
- ✅ **Dry-run**: Simulate changes without system modification
- ✅ **Apply**: Execute planned actions with progress tracking
- ✅ **Rollback**: Automatic reversion on failure
- ✅ **Report**: Structured outcome reporting in JSON/YAML

### Service Module Capabilities
- Service installation detection
- Systemd service state management (start, stop, enable, disable)
- Port conflict detection
- Configuration validation
- Package installation/removal (via system package manager)

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────┐
│                    CLI Frontend                     │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│                    API Layer                        │
│          (Intent Validation & Routing)              │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│                  Core Engine                        │
│   • State Inspector    • Action Planner             │
│   • Dry-run Engine     • Execution Controller       │
│   • Rollback Manager   • Report Generator           │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│                 Service Module                      │
│   • State Detection    • Action Translation         │
│   • Validation Logic   • System Tool Wrapper        │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│           System Tools (systemctl, etc)             │
└─────────────────────────────────────────────────────┘
```

## 🚀 Quick Start

### Prerequisites
- Rust 1.75 or later
- Linux system with systemd
- Root or sudo access (for system modifications)

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/yast3-prototype
cd yast3-prototype

# Build the project
cargo build --release

# Run tests
cargo test
```

### Basic Usage

```bash
# Inspect current state and plan changes (no system modification)
sudo ./target/release/yast3 inspect config.yaml

# Simulate changes (dry-run)
sudo ./target/release/yast3 dry-run config.yaml

# Apply changes with automatic rollback on failure
sudo ./target/release/yast3 apply config.yaml

# View structured report
sudo ./target/release/yast3 report --last
```

### Example Configuration

```yaml
service: nginx
state:
  installed: true
  enabled: true
  running: true
  port: 80
```

### Example Output

```
┌─ Inspection ─────────────────────────────────────┐
│ nginx: not installed                             │
│ service: not running                             │
│ port 80: available                               │
└──────────────────────────────────────────────────┘

┌─ Planned Actions ────────────────────────────────┐
│ 1. Install nginx package                         │
│ 2. Enable nginx service at boot                  │
│ 3. Start nginx service                           │
└──────────────────────────────────────────────────┘

┌─ Dry-run ────────────────────────────────────────┐
│ ✓ Would install nginx                            │
│ ✓ Would enable service at boot                   │
│ ✓ Would start service                            │
│                                                   │
│ No conflicts detected                            │
└──────────────────────────────────────────────────┘

Apply changes? [y/N]: y

┌─ Execution ──────────────────────────────────────┐
│ [1/3] Installing nginx... ✓                      │
│ [2/3] Enabling service... ✓                      │
│ [3/3] Starting service... ✓                      │
│                                                   │
│ Duration: 842ms                                  │
│ Status: SUCCESS                                  │
└──────────────────────────────────────────────────┘
```

## 📊 Technical Highlights

### Safety Guarantees
- **Pre-flight Validation**: All checks before state modification
- **Atomic Operations**: Each action is a discrete, trackable unit
- **Automatic Rollback**: Generated before execution, triggered on failure
- **Idempotent Actions**: Safe to run multiple times

### Performance Characteristics
- **Minimal Overhead**: Planning and validation are fast
- **Efficient Execution**: Delegates to native system tools
- **Memory Safe**: Written in Rust, no runtime exceptions
- **Concurrent Planning**: Action graph analysis in parallel (future optimization)

### Code Quality
- Comprehensive unit and integration tests
- Error handling with detailed context propagation
- Clean separation of concerns (core vs modules)
- Documented public APIs

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test suite
cargo test service_module::

# Integration tests (requires root)
sudo cargo test --test integration -- --test-threads=1
```

## 📈 Future Enhancements

### Phase 1 Expansion
- [ ] Additional modules (networking, users, packages, files)
- [ ] Configuration file format validation
- [ ] Transaction logging and audit trail
- [ ] Multi-service dependency resolution

### Phase 2 Features
- [ ] Distributed coordination for cluster management
- [ ] GraphQL API for programmatic access
- [ ] Web-based dashboard for monitoring
- [ ] Plugin system for custom modules
- [ ] Advanced policy engine

## 📚 Documentation

- [System Design Overview](docs/System_Design_Overview.pdf) - Complete architectural specification
- [Service Module Design](docs/service_module.md) - Detailed module documentation
- [API Reference](docs/api.md) - Core API documentation
- [Development Guide](docs/development.md) - Contributing guidelines

## 🤝 Research Collaboration

This prototype was developed to explore safe system configuration patterns and validate theoretical models in practice. I'm actively seeking opportunities to:

- Collaborate with research teams on distributed systems and safety-critical software
- Extend the prototype with formal verification methods
- Integrate with existing configuration management ecosystems
- Publish findings on practical safety models in system administration

**Areas of Interest**: Declarative systems, failure handling, state reconciliation, formal methods, infrastructure automation

## 🔗 References

This work draws inspiration from:

- **Declarative Configuration**: Kubernetes, Terraform, NixOS
- **Safety Models**: Rust's ownership system, database ACID properties
- **Operational Workflows**: Ansible's check mode, Docker's dry-run capabilities
- **State Management**: Control theory, desired state reconciliation

## 📬 Contact

**Author**: [Your Name]  
**Email**: your.email@example.com  
**LinkedIn**: [Your Profile]  
**Research Interests**: System safety, distributed systems, infrastructure automation

---

## 📄 License

This prototype is released under the MIT License. See [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

Special thanks to:
- **Grandle** and **Asperine** for the original system design
- The Rust community for excellent tooling and libraries
- Reviewers who provided feedback on safety model implementation

---

**Note**: This is a research prototype demonstrating core concepts. It is not production-ready and should be used for evaluation and experimentation only.
