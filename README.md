<p align="center">
<!-- product name logo -->
  <img width=600 src="https://user-images.githubusercontent.com/5758427/231206749-8f146b97-3c5a-4201-8388-3ffa88580415.png">
</p>
<hr/>
<p align="center">
  <a href="https://docs.zama.ai/tfhe-rs"> 📒 Read documentation</a> | <a href="https://zama.ai/community"> 💛 Community support</a> | <a href="https://github.com/zama-ai/awesome-zama"> 📚 FHE resources</a>
</p>
<p align="center">
<!-- Version badge using shields.io -->
  <a href="https://github.com/zama-ai/tfhe-rs/releases">
    <img src="https://img.shields.io/github/v/release/zama-ai/tfhe-rs?style=flat-square">
  </a>
  <!-- Link to tutorials badge using shields.io -->
  <a href="#license">
    <img src="https://img.shields.io/badge/License-BSD--3--Clause--Clear-orange?style=flat-square">
  </a>
<!-- Zama Bounty Program -->
  <a href="https://github.com/zama-ai/bounty-program">
    <img src="https://img.shields.io/badge/Contribute-Zama%20Bounty%20Program-yellow?style=flat-square">
  </a>
</p>



## Table of Contents
- **[About](#about)**
  - [What is TFHE-rs](#-what-is-tfhe-rs)
  - [Main features](#-main-features)
- **[Getting Started](#getting-started)**
   - [Configuration](#-configuration)
   - [A simple example](#-a-simple-example)
- **[Resources](#resources)**
   - [Tutorials](#-tutorials)
   - [Documentation](#-documentation)
- **[Working with Concrete ML](working-with-concrete-ml)**
   - [Disclaimers](#-disclaimers)
   - [Citations](#-citations)
   - [Contributing](#-contributing)
   - [License](#-license)
- **[Support](#support)**
<br></br>


## About

### 🟨 What is TFHE-rs

TFHE-rs is a pure Rust implementation of TFHE for boolean and integer arithmetics over encrypted data. It includes:
 - a **Rust** API
 - a **C** API
 - and a **client-side WASM** API

TFHE-rs is meant for developers and researchers who want full control over
what they can do with TFHE, while not having to worry about the low level
implementation. The goal is to have a stable, simple, high-performance, and
production-ready library for all the advanced features of TFHE.
<br></br>

### 🟨 Main features
TFHE-rs comes packed with powerful features, such as:

- A low-level, cryptographic library that implements Zama’s variant of TFHE, including programmable bootstrapping;
- An implementation of the original TFHE boolean API, which can be used as a drop-in replacement for other TFHE libraries;
- A short integer API, which enables exact, unbounded FHE integer arithmetics with up to 8 bits of message space;
- A public key encryption implementation for TFHE;
- Ciphertext and server key compression for efficient data transfer;
- A full Rust API, C bindings to the Rust High Level API, and a client-side Javascript API using WASM.

*Learn more features in TFHE-rs's [documentation](https://docs.zama.ai/tfhe-rs/readme).*
<br></br>

<p align="right">
  <a href="#table-of-contents" > ↑ Back to top </a> 
</p>



## Getting Started

### 🟨 Configuration
To use the latest version of `TFHE-rs` in your project, you first need to add it as a dependency in your `Cargo.toml`:

+ For x86_64-based machines running Unix-like OSes:

```toml
tfhe = { version = "*", features = ["boolean", "shortint", "integer", "x86_64-unix"] }
```

+ For Apple Silicon or aarch64-based machines running Unix-like OSes:

```toml
tfhe = { version = "*", features = ["boolean", "shortint", "integer", "aarch64-unix"] }
```

+ For x86_64-based machines with the [`rdseed instruction`](https://en.wikipedia.org/wiki/RDRAND) running Windows:

```toml
tfhe = { version = "*", features = ["boolean", "shortint", "integer", "x86_64"] }
```

> Note: You need to use a Rust version >= 1.72 to compile TFHE-rs.

> Note: aarch64-based machines are not yet supported for Windows as it's currently missing an entropy source to be able to seed the [CSPRNGs](https://en.wikipedia.org/wiki/Cryptographically_secure_pseudorandom_number_generator) used in TFHE-rs.

<br></br>

## 🟨 A simple example

Here is a full example:

``` rust
use tfhe::prelude::*;
use tfhe::{generate_keys, set_server_key, ConfigBuilder, FheUint32, FheUint8};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Basic configuration to use homomorphic integers
    let config = ConfigBuilder::default().build();

    // Key generation
    let (client_key, server_keys) = generate_keys(config);

    let clear_a = 1344u32;
    let clear_b = 5u32;
    let clear_c = 7u8;

    // Encrypting the input data using the (private) client_key
    // FheUint32: Encrypted equivalent to u32
    let mut encrypted_a = FheUint32::try_encrypt(clear_a, &client_key)?;
    let encrypted_b = FheUint32::try_encrypt(clear_b, &client_key)?;

    // FheUint8: Encrypted equivalent to u8
    let encrypted_c = FheUint8::try_encrypt(clear_c, &client_key)?;

    // On the server side:
    set_server_key(server_keys);

    // Clear equivalent computations: 1344 * 5 = 6720
    let encrypted_res_mul = &encrypted_a * &encrypted_b;

    // Clear equivalent computations: 1344 >> 5 = 42
    encrypted_a = &encrypted_res_mul >> &encrypted_b;

    // Clear equivalent computations: let casted_a = a as u8;
    let casted_a: FheUint8 = encrypted_a.cast_into();

    // Clear equivalent computations: min(42, 7) = 7
    let encrypted_res_min = &casted_a.min(&encrypted_c);

    // Operation between clear and encrypted data:
    // Clear equivalent computations: 7 & 1 = 1
    let encrypted_res = encrypted_res_min & 1_u8;

    // Decrypting on the client side:
    let clear_res: u8 = encrypted_res.decrypt(&client_key);
    assert_eq!(clear_res, 1_u8);

    Ok(())
}
```

To run this code, use the following command: 
<p align="center"> <code> cargo run --release </code> </p>

> Note that when running code that uses `tfhe-rs`, it is highly recommended
to run in release mode with cargo's `--release` flag to have the best performances possible.

*This example is explained in more detail in [this part of the documentation](docs/getting-started/quick_start)*

<p align="right">
  <a href="#table-of-contents" > ↑ Back to top </a> 
</p>



## Resources 

### 🟨 Tutorials
To be udpated
<br></br>
### 🟨 Documentation

Full, comprehensive documentation is available here: [https://docs.zama.ai/concrete-ml](https://docs.zama.ai/tfhe-rs).
<p align="right">
  <a href="#table-of-contents" > ↑ Back to top </a> 
</p>

## Working with TFHE-rs

### 🟨 Disclaimers

### Security Estimation

Security estimations are done using the
[Lattice Estimator](https://github.com/malb/lattice-estimator)
with `red_cost_model = reduction.RC.BDGL16`.

When a new update is published in the Lattice Estimator, we update parameters accordingly.

### Side-Channel Attacks

Mitigation for side channel attacks have not yet been implemented in TFHE-rs,
and will be released in upcoming versions.
<br></br>

### 🟨 Citations
To cite TFHE-rs in academic papers, please use the following entry:

```text
@Misc{TFHE-rs,
  title={{TFHE-rs: A Pure Rust Implementation of the TFHE Scheme for Boolean and Integer Arithmetics Over Encrypted Data}},
  author={Zama},
  year={2022},
  note={\url{https://github.com/zama-ai/tfhe-rs}},
}
```

### 🟨 Contributing

There are two ways to contribute to TFHE-rs:

- you can open issues to report bugs or typos, or to suggest new ideas
- you can ask to become an official contributor by emailing [hello@zama.ai](mailto:hello@zama.ai).
(becoming an approved contributor involves signing our Contributor License Agreement (CLA))

Only approved contributors can send pull requests, so please make sure to get in touch before you do!
<br></br>

### 🟨 License
This software is distributed under the **BSD-3-Clause-Clear** license. If you have any questions, please contact us at hello@zama.ai.
<p align="right">
  <a href="#table-of-contents" > ↑ Back to top </a> 
</p>


## Support

<a target="_blank" href="https://community.zama.ai">
  <img src="https://github.com/zama-ai/concrete-ml/assets/157474013/9d518f66-30da-4738-a154-a4d9fce93704">
</a>
<br></br>


🌟 If you find this project helpful or interesting, please consider giving it a star on GitHub! Your support helps to grow the community and motivates further development. 🚀

[![GitHub stars](https://img.shields.io/github/stars/zama-ai/concrete-ml.svg?style=social&label=Star)](https://github.com/zama-ai/concrete-ml)

Thank you! 
<p align="right">
  <a href="#table-of-contents" > ↑ Back to top </a> 
</p>
