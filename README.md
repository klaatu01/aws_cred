# `aws_cred`: AWS Credentials Management Library

`aws_cred` is a Rust library that provides intuitive management of AWS credentials. Store and edit your AWS credentials in the standard format with ease, removing the need for manual file editing.

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
aws_cred = "0.1.0"
```

Then, run the following to compile:

```bash
$ cargo build
```

## How to Use

```rust
use aws_cred::*;

let mut credentials = AWSCredentials::load().unwrap();
credentials
    .with_profile("default")
    .set_access_key_id("ACCESS_KEY")
    .set_secret_access_key("SECRET_KEY");
credentials.write().unwrap();
```

For a detailed exploration and additional samples, refer to the [API docs](https://docs.rs/aws_cred/0.0.1/aws_cred).
