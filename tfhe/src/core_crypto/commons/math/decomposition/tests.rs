use crate::core_crypto::algorithms::misc::{
    divide_ceil, divide_round, modular_distance_custom_mod,
};
use crate::core_crypto::commons::ciphertext_modulus::CiphertextModulus;
use crate::core_crypto::commons::math::decomposition::{
    SignedDecomposer, SignedDecomposerNonNative,
};
use crate::core_crypto::commons::math::random::{RandomGenerable, Uniform};
use crate::core_crypto::commons::math::torus::UnsignedTorus;
use crate::core_crypto::commons::numeric::{Numeric, SignedInteger, UnsignedInteger};
use crate::core_crypto::commons::parameters::{DecompositionBaseLog, DecompositionLevelCount};
use crate::core_crypto::commons::test_tools::{any_uint, any_usize, random_usize_between};
use std::fmt::Debug;

fn valid_decomposers<T: UnsignedInteger>() -> Vec<SignedDecomposer<T>> {
    let mut valid_decomposers = vec![];
    for base_log in (1..T::BITS).map(DecompositionBaseLog) {
        for level_count in (1..T::BITS).map(DecompositionLevelCount) {
            if base_log.0 * level_count.0 < T::BITS {
                valid_decomposers.push(SignedDecomposer::new(base_log, level_count));
                continue;
            }

            // If the current base_log * level_count exceeds T::BITS then as level_count increases
            // all the decomposers after it won't be valid, so break
            break;
        }
    }

    valid_decomposers
}

fn test_decompose_recompose<T: UnsignedInteger + Debug + RandomGenerable<Uniform>>()
where
    <T as UnsignedInteger>::Signed: Debug + SignedInteger,
{
    let valid_decomposers = valid_decomposers::<T>();
    let runs_per_decomposer = divide_ceil(100_000, valid_decomposers.len());

    for decomposer in valid_decomposers {
        for _ in 0..runs_per_decomposer {
            let input = any_uint::<T>();

            // Decompose/recompose test
            for (term_idx, term) in decomposer.decompose(input).enumerate() {
                assert_eq!(term.level().0, decomposer.level_count - term_idx);
                let signed_term = term.value().into_signed();
                // Shift by base_log - 1 directly to avoid overflows
                let half_basis = T::Signed::ONE << (decomposer.base_log - 1);
                assert!(
                    -half_basis <= signed_term,
                    "-half_basis={:?}, signed_term = {signed_term:?}",
                    -half_basis,
                );
                assert!(
                    signed_term <= half_basis,
                    "signed_term={signed_term:?}, half_basis = {half_basis:?}",
                );
            }
            let closest = decomposer.closest_representable(input);
            assert_eq!(
                closest,
                decomposer.recompose(decomposer.decompose(closest)).unwrap()
            );
        }
    }
}

#[test]
fn test_decompose_recompose_u32() {
    test_decompose_recompose::<u32>();
}

#[test]
fn test_decompose_recompose_u64() {
    test_decompose_recompose::<u64>();
}

fn test_round_to_closest_representable<T: UnsignedTorus>() {
    let valid_decomposers = valid_decomposers::<T>();
    let runs_per_decomposer = divide_ceil(100_000, valid_decomposers.len());

    // Checks that the decomposing and recomposing a value brings the closest representable
    for decomposer in valid_decomposers {
        for _ in 0..runs_per_decomposer {
            let input = any_uint::<T>();

            let rounded = decomposer.closest_representable(input);

            let epsilon =
                (T::ONE << (T::BITS - (decomposer.base_log * decomposer.level_count) - 1)) / T::TWO;
            // Adding/removing an epsilon should not change the closest representable
            assert_eq!(
                rounded,
                decomposer.closest_representable(rounded.wrapping_add(epsilon))
            );
            assert_eq!(
                rounded,
                decomposer.closest_representable(rounded.wrapping_sub(epsilon))
            );
        }
    }
}

#[test]
fn test_round_to_closest_representable_u32() {
    test_round_to_closest_representable::<u32>();
}

#[test]
fn test_round_to_closest_representable_u64() {
    test_round_to_closest_representable::<u64>();
}

fn test_round_to_closest_twice<T: UnsignedTorus + Debug>() {
    let valid_decomposers = valid_decomposers::<T>();
    let runs_per_decomposer = divide_ceil(100_000, valid_decomposers.len());

    for decomposer in valid_decomposers {
        for _ in 0..runs_per_decomposer {
            let input = any_uint::<T>();

            // Round twice test, should not change the returned value
            let rounded_once = decomposer.closest_representable(input);
            let rounded_twice = decomposer.closest_representable(rounded_once);
            assert_eq!(rounded_once, rounded_twice);
        }
    }
}

#[test]
fn test_round_to_closest_twice_u32() {
    test_round_to_closest_twice::<u32>();
}

#[test]
fn test_round_to_closest_twice_u64() {
    test_round_to_closest_twice::<u64>();
}

// Return a random decomposition valid for the size of the T type.
fn random_decomp_non_native<T: UnsignedInteger>(
    ciphertext_modulus: CiphertextModulus<T>,
) -> SignedDecomposerNonNative<T> {
    let mut base_log;
    let mut level_count;
    loop {
        base_log = random_usize_between(1..T::BITS);
        level_count = random_usize_between(1..T::BITS);
        if base_log * level_count < T::BITS {
            break;
        }
    }
    SignedDecomposerNonNative::new(
        DecompositionBaseLog(base_log),
        DecompositionLevelCount(level_count),
        ciphertext_modulus,
    )
}

fn test_round_to_closest_representable_non_native<T: UnsignedTorus>(
    ciphertext_modulus: CiphertextModulus<T>,
) {
    {
        let log_b = any_usize();
        let level_max = any_usize();
        let bits = T::BITS;
        let base_log = (log_b % ((bits / 4) - 1)) + 1;
        let level_count = (level_max % 4) + 1;

        let val = T::ZERO;
        let smallest_representable = divide_round(
            ciphertext_modulus.get_custom_modulus(),
            1u128 << (base_log * level_count),
        );
        let sub_smallest_representable = T::cast_from(smallest_representable / 2);
        let epsilon = any_uint::<T>() % sub_smallest_representable;
        let ciphertext_modulus_as_t = T::cast_from(ciphertext_modulus.get_custom_modulus());

        let decomposer = SignedDecomposerNonNative::new(
            DecompositionBaseLog(base_log),
            DecompositionLevelCount(level_count),
            ciphertext_modulus,
        );

        let val_plus_epsilon = val.wrapping_add_custom_mod(epsilon, ciphertext_modulus_as_t);
        for (term_idx, term) in decomposer.decompose(val_plus_epsilon).enumerate() {
            assert_eq!(term.level().0, level_count - term_idx);
            let term = term.value();
            let half_basis = (T::ONE << decomposer.base_log) / T::TWO;

            let abs_term = if term > ciphertext_modulus_as_t / T::TWO {
                ciphertext_modulus_as_t - term
            } else {
                term
            };

            assert!(abs_term <= half_basis);
        }
        assert_eq!(val, decomposer.closest_representable(val_plus_epsilon));

        let val_minus_epsilon = val.wrapping_sub_custom_mod(epsilon, ciphertext_modulus_as_t);
        for (term_idx, term) in decomposer.decompose(val_minus_epsilon).enumerate() {
            assert_eq!(term.level().0, level_count - term_idx);
            let term = term.value();
            let half_basis = (T::ONE << decomposer.base_log) / T::TWO;

            let abs_term = if term > ciphertext_modulus_as_t / T::TWO {
                ciphertext_modulus_as_t - term
            } else {
                term
            };

            assert!(abs_term <= half_basis);
        }
        assert_eq!(val, decomposer.closest_representable(val_minus_epsilon));
    }

    for _ in 0..1000 {
        let log_b = any_usize();
        let level_max = any_usize();
        let val = any_uint::<T>();
        let bits = T::BITS;
        let base_log = (log_b % ((bits / 4) - 1)) + 1;
        let level_count = (level_max % 4) + 1;
        let rep_bits: usize = base_log * level_count;

        let val = val << (bits - rep_bits);
        let smallest_representable = divide_round(
            ciphertext_modulus.get_custom_modulus(),
            1u128 << (base_log * level_count),
        );
        let sub_smallest_representable = T::cast_from(smallest_representable / 2);
        let epsilon = any_uint::<T>() % sub_smallest_representable;
        let ciphertext_modulus_as_t = T::cast_from(ciphertext_modulus.get_custom_modulus());

        let decomposer = SignedDecomposerNonNative::new(
            DecompositionBaseLog(base_log),
            DecompositionLevelCount(level_count),
            ciphertext_modulus,
        );

        {
            let val_plus_epsilon = val.wrapping_add_custom_mod(epsilon, ciphertext_modulus_as_t);
            for (term_idx, term) in decomposer.decompose(val_plus_epsilon).enumerate() {
                assert_eq!(term.level().0, level_count - term_idx);
                let term = term.value();
                let half_basis = (T::ONE << decomposer.base_log) / T::TWO;

                let abs_term = if term > ciphertext_modulus_as_t / T::TWO {
                    ciphertext_modulus_as_t - term
                } else {
                    term
                };

                assert!(abs_term <= half_basis);
            }
            let closest = decomposer.closest_representable(val_plus_epsilon);
            let distance = modular_distance_custom_mod(val, closest, ciphertext_modulus_as_t);

            // Test succesful
            if distance == T::ZERO {
                continue;
            }

            // -1 as we don't divide exactly for prime Q
            let max_correct_bits = base_log * level_count - 1;
            let max_err_bits = (T::BITS - max_correct_bits) as u32;
            assert!(
            distance.ilog2() <= max_err_bits,
            "base_log={base_log}, level_count={level_count}, val={val:064b}, closest={closest:064b}"
        );
        }

        {
            let val_minus_epsilon = val.wrapping_sub_custom_mod(epsilon, ciphertext_modulus_as_t);
            for (term_idx, term) in decomposer.decompose(val_minus_epsilon).enumerate() {
                assert_eq!(term.level().0, level_count - term_idx);
                let term = term.value();
                let half_basis = (T::ONE << decomposer.base_log) / T::TWO;

                let abs_term = if term > ciphertext_modulus_as_t / T::TWO {
                    ciphertext_modulus_as_t - term
                } else {
                    term
                };

                assert!(abs_term <= half_basis);
            }
            let closest = decomposer.closest_representable(val_minus_epsilon);
            let distance = modular_distance_custom_mod(val, closest, ciphertext_modulus_as_t);

            // Test succesful
            if distance == T::ZERO {
                continue;
            }

            // -1 as we don't divide exactly for prime Q
            let max_correct_bits = base_log * level_count - 1;
            let max_err_bits = (T::BITS - max_correct_bits) as u32;
            assert!(
            distance.ilog2() <= max_err_bits,
            "base_log={base_log}, level_count={level_count}, val={val:064b}, closest={closest:064b}"
        );
        }
    }
}

#[test]
fn test_round_to_closest_representable_non_native_u64() {
    test_round_to_closest_representable_non_native::<u64>(
        CiphertextModulus::try_new((1 << 64) - (1 << 32) + 1).unwrap(),
    );
}

fn test_round_to_closest_twice_non_native<T: UnsignedTorus + Debug>(
    ciphertext_modulus: CiphertextModulus<T>,
) {
    for _ in 0..1000 {
        let decomp = random_decomp_non_native(ciphertext_modulus);
        let input: T = any_uint();

        let rounded_once = decomp.closest_representable(input);
        let rounded_twice = decomp.closest_representable(rounded_once);
        assert_eq!(rounded_once, rounded_twice);
    }
}

#[test]
fn test_round_to_closest_twice_non_native_u64() {
    test_round_to_closest_twice_non_native::<u64>(
        CiphertextModulus::try_new((1 << 64) - (1 << 32) + 1).unwrap(),
    );
}

#[test]
fn test_decomposer_mod_smaller_2_to_63() {
    let ciphertext_modulus = CiphertextModulus::<u64>::try_new(1 << 62).unwrap();
    // 0010_0111_10...0
    let value_to_decompose_non_native =
        (1u64 << 61) + (1 << 58) + (1 << 57) + (1 << 56) + (1 << 55);
    let base_log = DecompositionBaseLog(3);
    let level_count = DecompositionLevelCount(2);

    let non_native_decomposer =
        SignedDecomposerNonNative::new(base_log, level_count, ciphertext_modulus);
    let non_native_closest =
        non_native_decomposer.closest_representable(value_to_decompose_non_native);
    // 0010_1000_00...0
    assert_eq!(non_native_closest, (1u64 << 61) + (1 << 59));
    let non_native_decomp_iter = non_native_decomposer.decompose(value_to_decompose_non_native);

    // Check we get the same results shifted when computing on the shifted value to fill the MSBs
    let value_to_decompose_native = value_to_decompose_non_native << 2;
    let native_decomposer = SignedDecomposer::new(base_log, level_count);
    let native_closest = native_decomposer.closest_representable(value_to_decompose_native);
    assert_eq!(non_native_closest << 2, native_closest);

    let native_decomp_iter = native_decomposer.decompose(value_to_decompose_native);

    for (non_native_term, native_term) in non_native_decomp_iter.zip(native_decomp_iter) {
        assert_eq!(
            non_native_term.to_recomposition_summand() << 2,
            native_term.to_recomposition_summand()
        );
    }
}

fn test_decompose_recompose_non_native<T: UnsignedInteger + Debug + RandomGenerable<Uniform>>(
    ciphertext_modulus: CiphertextModulus<T>,
) where
    <T as UnsignedInteger>::Signed: Debug + SignedInteger,
{
    let ciphertext_modulus_as_t = T::cast_from(ciphertext_modulus.get_custom_modulus());
    // Checks that the decomposing and recomposing a value brings the closest representable
    for _ in 0..100_000 {
        let decomposer = random_decomp_non_native::<T>(ciphertext_modulus);
        let base_log = decomposer.base_log().0;
        let level_count = decomposer.level_count().0;
        let input = any_uint::<T>() % ciphertext_modulus_as_t;

        for (term_idx, term) in decomposer.decompose(input).enumerate() {
            assert_eq!(term.level().0, level_count - term_idx);
            let term = term.value();
            let half_basis = (T::ONE << decomposer.base_log) / T::TWO;

            let abs_term = if term > ciphertext_modulus_as_t / T::TWO {
                ciphertext_modulus_as_t - term
            } else {
                term
            };

            assert!(abs_term <= half_basis);
        }
        let closest = decomposer.closest_representable(input);
        let distance = modular_distance_custom_mod(input, closest, ciphertext_modulus_as_t);

        // Test succesful
        if distance == T::ZERO {
            continue;
        }

        // -1 as we don't divide exactly for prime Q
        let max_correct_bits = base_log * level_count - 1;
        let max_err_bits = (T::BITS - max_correct_bits) as u32;
        assert!(
        distance.ilog2() <= max_err_bits,
        "base_log={base_log}, level_count={level_count}, input={input:064b}, closest={closest:064b}"
    );
    }
}

#[test]
pub fn test_decompose_recompose_non_native_u64() {
    test_decompose_recompose_non_native::<u64>(
        CiphertextModulus::try_new((1 << 64) - (1 << 32) + 1).unwrap(),
    )
}
