# `aws_cred`: AWS Credentials Manipulation Library

`aws_cred` is a Rust library that provides intuitive manipulation of AWS credentials. Store and edit your AWS credentials in the standard format with ease, removing the need for manual file editing.

## Features

- **Simple Loading**: Extract AWS credentials from the default path or specify your own.
- **Profile Modification**: Utilize the fluent API to effortlessly alter or introduce new profiles.
- **Instant Saving**: Save your modifications back to the credentials file with a single command.

## Getting Started

### Prerequisites

Ensure you have Rust and Cargo already set up. If not, install them from [rustup.rs](https://rustup.rs/).

### Installation

Integrate `aws_cred` into your project by adding the following line in your `Cargo.toml`:

```toml
[dependencies]
aws_cred = "0.0.3"
```

Then, run the following to compile:

```bash
$ cargo build
```

## How to Use

```rust
let mut credentials = AWSCredentials::load()?;
credentials
    .with_profile("default")
    .set_access_key_id("ACCESS_KEY")
    .set_secret_access_key("SECRET_KEY");
credentials.write()?;
```

For a detailed exploration and additional samples, refer to the [API docs](https://docs.rs/aws_cred/0.0.1/aws_cred).

## License

This crate is distributed under the terms of the MIT license. See the [LICENSE](https://opensource.org/licenses/MIT) file for details.
