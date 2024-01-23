# Making rayon and tfhe-rs work together

[rayon](https://crates.io/crates/rayon) is a popular create to easily write multi-threaded code in Rust.

It is possible to use rayon to write multi-threaded tfhe-rs code. However due to internal details of `rayon` and
`tfhe-rs`, there is some special setup that needs to be done.

## Single Client Application

### The problem 

The high level api requires to call `set_server_key` on each thread where computations needs to be done.
So a first attempt at using rayon with tfhe-rs might look like this:

```rust
use rayon::prelude::*;
use tfhe::prelude::*;
use tfhe::{ConfigBuilder, set_server_key, FheUint8, generate_keys};

fn main() {
    let (cks, sks) = generate_keys(ConfigBuilder::default());
    
    let xs = [
        FheUint8::encrypt(1u8, &cks),
        FheUint8::encrypt(2u8, &cks),
    ];

    let ys = [
        FheUint8::encrypt(3u8, &cks),
        FheUint8::encrypt(4u8, &cks),
    ];


    // set_server_key in each closure as they might be
    // running in different threads
    let (a, b) = rayon::join(
      || {
          set_server_key(sks.clone());
          &xs[0] + &ys[0]
      },
      || {
          set_server_key(sks.clone());
          &xs[1] + &ys[1]
      }
    );
}
```

However, due to rayon's work stealing mechanism and tfhe-rs's internals, this may create `BorrowMutError'.


### Working example

The correct way is to call `rayon::broadcast`

```rust
use rayon::prelude::*;
use tfhe::prelude::*;
use tfhe::{ConfigBuilder, set_server_key, FheUint8, generate_keys};

fn main() {
    let (cks, sks) = generate_keys(ConfigBuilder::default());
    
    // set the server key in all of the rayon's threads so that
    // we won't need to do it later
    rayon::broadcast(|_| set_server_key(sks.clone()));
    // Set the server key in the main thread
    set_server_key(sks);
    
    let xs = [
        FheUint8::encrypt(1u8, &cks),
        FheUint8::encrypt(2u8, &cks),
    ];

    let ys = [
        FheUint8::encrypt(3u8, &cks),
        FheUint8::encrypt(4u8, &cks),
    ];

    let (a, b) = rayon::join(
      || {
          &xs[0] + &ys[0]
      },
      || {
          &xs[1] + &ys[1]
      }
    );
}
```


## Multi-Client Applications

If you application needs to operate on data from different clients concurently, and that you want each client to use 
multiple threads, you will need to create different rayon thread pools

```rust
use rayon::prelude::*;
use tfhe::prelude::*;
use tfhe::{ConfigBuilder, set_server_key, FheUint8, generate_keys};

fn main() {
    let (cks1, sks1) = generate_keys(ConfigBuilder::default());
    let xs1 = [
        FheUint8::encrypt(1u8, &cks1),
        FheUint8::encrypt(2u8, &cks1),
    ];

    let ys1 = [
        FheUint8::encrypt(3u8, &cks1),
        FheUint8::encrypt(4u8, &cks1),
    ];

    let (cks2, sks2) = generate_keys(ConfigBuilder::default());
    let xs2 = [
        FheUint8::encrypt(100u8, &cks2),
        FheUint8::encrypt(200u8, &cks2),
    ];

    let ys2 = [
        FheUint8::encrypt(103u8, &cks2),
        FheUint8::encrypt(204u8, &cks2),
    ];

    let client_1_pool = rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap();
    let client_2_pool = rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap();
    
    client_1_pool.broadcast(|_| set_server_key(sks1.clone()));
    client_2_pool.broadcast(|_| set_server_key(sks2.clone()));
    
    rayon::join(|| {
        client_1_pool.install(|| {
            let (a1, b1) = rayon::join(
                || {
                    &xs1[0] + &ys1[0]
                },
                || {
                    &xs1[1] + &ys1[1]
                }
            );
        });
    }, || {
        client_2_pool.install(|| {
            let (a2, b2) = rayon::join(
                || {
                    &xs2[0] + &ys2[0]
                },
                || {
                    &xs2[1] + &ys2[1]
                }
            );
        })
    });
}
```

This can be useful if you have some rust `#[test]`

```Rust
// Pseudo code
#[test]
fn test_1() {
    let pool = rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap();
    pool.broadcast(|_| set_server_key(sks1.clone()));
    pool.install(|| {
        let result = call_to_a_multithreaded_function(...);
        assert_eq!(result, expected_value);
    })
}

#[test]
fn test_2() {
    let pool = rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap();
    pool.broadcast(|_| set_server_key(sks1.clone()));
    pool.install(|| {
        let result = call_to_another_multithreaded_function(...);
        assert_eq!(result, expected_value);
    })
}
```