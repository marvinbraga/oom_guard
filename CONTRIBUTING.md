# Contributing to OOM Guard

Thank you for considering contributing to OOM Guard! This document provides guidelines and instructions for contributing.

## 🎯 Ways to Contribute

- **Bug Reports:** Found a bug? [Open an issue](https://github.com/marvinbraga/oom_guard/issues/new)
- **Feature Requests:** Have an idea? [Start a discussion](https://github.com/marvinbraga/oom_guard/discussions)
- **Code Contributions:** Submit pull requests with improvements
- **Documentation:** Help improve docs, examples, or translations
- **Testing:** Test on different platforms and report results

## 🚀 Getting Started

### Prerequisites

- Rust 1.70 or higher
- Linux system with `/proc` filesystem
- Git

### Setup Development Environment

```bash
# Fork the repository on GitHub, then:
git clone https://github.com/YOUR_USERNAME/oom_guard.git
cd oom_guard

# Build
cargo build

# Run tests
cargo test

# Run in development mode
cargo run -- --dryrun -m 20 -d
```

## 📝 Pull Request Process

1. **Fork & Branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make Changes**
   - Write clear, concise code
   - Follow Rust conventions
   - Add tests for new functionality
   - Update documentation as needed

3. **Test Your Changes**
   ```bash
   # Run all tests
   cargo test

   # Check formatting
   cargo fmt --all -- --check

   # Run clippy
   cargo clippy -- -D warnings

   # Test in dry-run mode
   sudo cargo run -- --dryrun -m 20 -d
   ```

4. **Commit**
   - Use clear, descriptive commit messages
   - Follow conventional commits format:
     ```
     feat: add process group priority selection
     fix: correct swap threshold calculation
     docs: update installation instructions
     test: add unit tests for selector
     ```

5. **Push & Create PR**
   ```bash
   git push origin feature/your-feature-name
   ```
   - Open a pull request on GitHub
   - Fill in the PR template
   - Link related issues

## 🧪 Testing Guidelines

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Arrange
        let config = Config::default();

        // Act
        let result = my_function(&config);

        // Assert
        assert_eq!(result, expected);
    }
}
```

### Test Coverage

- Aim for >80% code coverage
- Test edge cases and error conditions
- Include integration tests for complex features

## 📚 Code Style

### Rust Style Guide

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Write idiomatic Rust code

### Documentation

- Document all public APIs with `///` comments
- Include examples in documentation:
  ```rust
  /// Selects a victim process based on configuration.
  ///
  /// # Examples
  ///
  /// ```
  /// let selector = ProcessSelector::new(config);
  /// let victim = selector.select_victim()?;
  /// ```
  pub fn select_victim(&self) -> Result<ProcessInfo> {
      // ...
  }
  ```

### Error Handling

- Use `anyhow::Result` for functions that can fail
- Provide context with `.context()`:
  ```rust
  File::open(path)
      .context(format!("Failed to open {}", path))?
  ```

## 🐛 Bug Reports

Good bug reports include:

- **Description:** Clear description of the bug
- **Steps to Reproduce:** Minimal steps to reproduce
- **Expected Behavior:** What should happen
- **Actual Behavior:** What actually happens
- **Environment:**
  - OS and version
  - OOM Guard version
  - Rust version (if building from source)
- **Logs:** Relevant log output
  ```bash
  sudo journalctl -u oom_guard -n 100 --no-pager
  ```

## 💡 Feature Requests

Good feature requests include:

- **Use Case:** Why is this feature needed?
- **Proposal:** How should it work?
- **Alternatives:** Other approaches considered?
- **Examples:** Similar features in other tools?

## 🔍 Code Review Process

All submissions require review. We look for:

- **Correctness:** Does it work as intended?
- **Tests:** Are there adequate tests?
- **Documentation:** Is it well documented?
- **Style:** Does it follow project conventions?
- **Performance:** Any performance implications?
- **Security:** Any security concerns?

## 📋 Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

**Example:**
```
feat(selector): add process group priority selection

Implement priority-based selection for process groups to improve
victim selection accuracy in containerized environments.

Closes #123
```

## 🏷️ Versioning

We use [Semantic Versioning](https://semver.org/):

- **MAJOR:** Incompatible API changes
- **MINOR:** Backwards-compatible functionality
- **PATCH:** Backwards-compatible bug fixes

## 📜 License

By contributing, you agree that your contributions will be licensed under the GPL-2.0 License.

## 🤝 Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inspiring community for all.

### Our Standards

**Positive behavior:**
- Using welcoming and inclusive language
- Being respectful of differing viewpoints
- Gracefully accepting constructive criticism
- Focusing on what is best for the community

**Unacceptable behavior:**
- Harassment, trolling, or derogatory comments
- Personal or political attacks
- Publishing others' private information
- Other conduct reasonably considered inappropriate

## 📞 Questions?

- **Discussions:** [GitHub Discussions](https://github.com/marvinbraga/oom_guard/discussions)
- **Email:** mvbraga@gmail.com

---

Thank you for contributing to OOM Guard! 🎉
