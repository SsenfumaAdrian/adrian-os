# Contributing to ADRIAN OS

## Contribution Principles
- Architecture before implementation
- Security review for privileged code
- Performance validation for hot paths
- Documentation required for public interfaces
- Tests required for non-trivial behavior
- Avoid architectural shortcuts

## Pull Request Expectations
Each pull request should include:
1. Problem statement
2. Design rationale
3. Security impact
4. Performance considerations
5. Testing notes
6. Future compatibility notes

## Coding Standards
- Rust for system components
- Dart for app/UI platform components
- Python only for tooling, AI workflows, testing, and automation
- Public APIs require documentation
- Unsafe Rust must be minimal and justified
