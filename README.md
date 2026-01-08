# lre

`lre` provides Rust bindings for the `libregexp` C library, which is a lightweight regular expression engine from QuickJS.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
lre = "0.1"
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

The underlying libregexp library is also MIT licensed.

## Acknowledgments

- [Fabrice Bellard](https://bellard.org/) for creating the original libregexp library
- The Rust community for excellent tooling (bindgen, cc, etc.)
- All contributors and users of this crate
