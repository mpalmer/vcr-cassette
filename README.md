<h1 align="center">vcr-cassette</h1>
<div align="center">
  <strong>
    Serializer and deserializer for the VCR Cassette format
  </strong>
</div>

<br />

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/vcr-cassette">
    <img src="https://img.shields.io/crates/v/vcr-cassette.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/vcr-cassette">
    <img src="https://img.shields.io/crates/d/vcr-cassette.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs.rs docs -->
  <a href="https://docs.rs/vcr-cassette">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
</div>

<div align="center">
  <h3>
    <a href="https://docs.rs/vcr-cassette">
      API Docs
    </a>
    <span> | </span>
    <a href="https://github.com/http-rs/vcr-cassette/releases">
      Releases
    </a>
    <span> | </span>
    <a href="https://github.com/http-rs/vcr-cassette/blob/master.github/CONTRIBUTING.md">
      Contributing
    </a>
  </h3>
</div>

## Examples

Given the following `.json` VCR Cassette recording:
```json
{
    "http_interactions": [
        {
            "request": {
                "uri": "http://localhost:7777/foo",
                "body": "",
                "method": "get",
                "headers": { "Accept-Encoding": [ "identity" ] }
            },
            "response": {
                "body": "Hello foo",
                "http_version": "1.1",
                "status": { "code": 200, "message": "OK" },
                "headers": {
                    "Date": [ "Thu, 27 Oct 2011 06:16:31 GMT" ],
                    "Content-Type": [ "text/html;charset=utf-8" ],
                    "Content-Length": [ "9" ],
                }
            },
            "recorded_at": "Tue, 01 Nov 2011 04:58:44 GMT"
        },
    ],
    "recorded_with": "VCR 2.0.0"
}
```

We can deserialize it using [`serde_json`](https://docs.rs/serde-json):

```rust
use std::fs;
use vcr_cassette::Cassette;

let example = fs::read_to_string("tests/fixtures/example.json").unwrap();
let cassette: Cassette = serde_json::from_str(&example).unwrap();
```

To deserialize `.yaml` Cassette files use
[`serde_yaml`](https://docs.rs/serde-yaml) instead.

## Installation
```sh
$ cargo add vcr-cassette
```

## Features

* `json` -- enables parsing and comparison of JSON request and response bodies.
  Saves having to escape every double quote character in your JSON-format bodies when you're manually
  writing them.  Looks like this:

  ```json
  {
    "body": {
      "json": {
        "arbitrary": ["json", "is", "now", "supported"],
        "success_factor": 100,
      }
    }
  }
  ```

* `matching` -- provides a mechanism for specifying "matchers" for request bodies, rather than a request body
  having to be byte-for-byte compatible with what's specified in the cassette.  There are currently two match types available, `substring` and `regex` (if the `regex` feature is also enabled).
  They do more-or-less what they say on the tin.  Use them like this:

  ```json
  {
    "body": {
      "matches": [
        { "substring": "something" },
        { "substring": "funny" },
        { "regex": "\\d+" }
      ]
    }
  }
  ```

  The above stanza, appropriately placed in a *request* specification, will match any request whose body contains the strings `"something"`, and `"funny"`, and *also* contains a number (of any length).

* `regex` -- Enables the `regex` match type.
  This is a separate feature, because the `regex` crate can be a bit heavyweight for resource-constrained environments, and so it's optional, in case you don't need it.

## Safety
This crate uses ``#![deny(unsafe_code)]`` to ensure everything is implemented in
100% Safe Rust.

## Contributing
Want to join us? Check out our ["Contributing" guide][contributing] and take a
look at some of these issues:

- [Issues labeled "good first issue"][good-first-issue]
- [Issues labeled "help wanted"][help-wanted]

[contributing]: https://github.com/http-rs/vcr-cassette/blob/master.github/CONTRIBUTING.md
[good-first-issue]: https://github.com/http-rs/vcr-cassette/labels/good%20first%20issue
[help-wanted]: https://github.com/http-rs/vcr-cassette/labels/help%20wanted

## License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br/>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
