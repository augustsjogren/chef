# Contributing

`chef` is a personal project shared as a showcase. It is not accepting pull requests, and issues may go unanswered.

You are welcome to fork it and adapt it to your own setup under the terms of the [MIT license](LICENSE).

## Local checks

If you work on a fork, run these from the repository root:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo package --locked
```

`chef` targets macOS and Linux.

## Security

See [SECURITY.md](SECURITY.md) for reporting vulnerabilities privately.
