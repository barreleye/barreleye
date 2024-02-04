# Contribute to Barreleye

Barreleye Insights is an open-source project administered by [Barreleye](https://barreleye.com/). We appreciate your interest and efforts to contribute to Barreleye Insights. See the [LICENSE](LICENSE) licensing information. All work done is available on GitHub.

We highly appreciate your effort to contribute, but we recommend you talk to a maintainer before spending a lot of time making a pull request that may not align with the project roadmap. Whether it is from Barreleye or contributors, every pull request goes through the same process.

## Feature Requests

Feature Requests by the community are highly encouraged. Feel free to submit a new one or upvote an existing feature request at [Github Discussions](https://github.com/barreleye/barreleye/discussions).

## Code of Conduct

This project, and everyone participating in it, are governed by the [Barreleye Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold it. Make sure to read the [full text](CODE_OF_CONDUCT.md) to understand which type of actions may or may not be tolerated.

## Bugs

Barreleye is using [GitHub issues](https://github.com/barreleye/barreleye/issues) to manage bugs. We keep a close eye on them. Before filing a new issue, try to ensure your problem does not already exist.

---

## Before Submitting a Pull Request

The Barreleye core team will review your pull request and either merge it, request changes, or close it.

## Contribution Prerequisites

- You have [Rust](https://www.rust-lang.org/) v1.70+ installed.
- You are familiar with [Git](https://git-scm.com).

**Before submitting your pull request** make sure the following requirements are fulfilled:

- Fork the repository and create your new branch from `main`.
- Run `cargo build` in the root of the repository.
- If you've fixed a bug or added code that should be tested, please make sure to add tests
- Check all by running:
  - `rustfmt **/*.rs`
  - `cargo test --all`
  - `cargo clippy --all`
  - `cargo +nightly udeps`
- If your contribution fixes an existing issue, please make sure to link it in your pull request.