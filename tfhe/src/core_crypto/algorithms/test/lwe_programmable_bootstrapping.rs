use super::*;
use crate::core_crypto::keycache::KeyCacheAccess;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[cfg(not(feature = "__coverage"))]
const NB_TESTS: usize = 10;
#[cfg(feature = "__coverage")]
const NB_TESTS: usize = 1;

pub fn generate_keys<
    Scalar: UnsignedTorus + Sync + Send + CastFrom<usize> + CastInto<usize> + Serialize + DeserializeOwned,
>(
    params: ClassicTestParams<Scalar>,
    rsc: &mut TestResources,
) -> ClassicBootstrapKeys<Scalar> {
    // Create the LweSecretKey
    let input_lwe_secret_key = allocate_and_generate_new_binary_lwe_secret_key(
        params.lwe_dimension,
        &mut rsc.secret_random_generator,
    );
    let output_glwe_secret_key = allocate_and_generate_new_binary_glwe_secret_key(
        params.glwe_dimension,
        params.polynomial_size,
        &mut rsc.secret_random_generator,
    );
    let output_lwe_secret_key = output_glwe_secret_key.clone().into_lwe_secret_key();

    let mut bsk = LweBootstrapKey::new(
        Scalar::ZERO,
        params.glwe_dimension.to_glwe_size(),
        params.polynomial_size,
        params.pbs_base_log,
        params.pbs_level,
        params.lwe_dimension,
        params.ciphertext_modulus,
    );

    par_generate_lwe_bootstrap_key(
        &input_lwe_secret_key,
        &output_glwe_secret_key,
        &mut bsk,
        params.glwe_modular_std_dev,
        &mut rsc.encryption_random_generator,
    );

    assert!(check_encrypted_content_respects_mod(
        &*bsk,
        params.ciphertext_modulus
    ));

    let mut fbsk = FourierLweBootstrapKey::new(
        params.lwe_dimension,
        params.glwe_dimension.to_glwe_size(),
        params.polynomial_size,
        params.pbs_base_log,
        params.pbs_level,
    );

    par_convert_standard_lwe_bootstrap_key_to_fourier(&bsk, &mut fbsk);

    ClassicBootstrapKeys {
        small_lwe_sk: input_lwe_secret_key,
        big_lwe_sk: output_lwe_secret_key,
        bsk,
        fbsk,
    }
}

fn lwe_encrypt_pbs_decrypt_custom_mod<Scalar>(params: ClassicTestParams<Scalar>)
where
    Scalar: UnsignedTorus
        + Sync
        + Send
        + CastFrom<usize>
        + CastInto<usize>
        + Serialize
        + DeserializeOwned,
    ClassicTestParams<Scalar>: KeyCacheAccess<Keys = ClassicBootstrapKeys<Scalar>>,
{
    let lwe_modular_std_dev = params.lwe_modular_std_dev;
    let ciphertext_modulus = params.ciphertext_modulus;
    let message_modulus_log = params.message_modulus_log;
    let msg_modulus = Scalar::ONE.shl(message_modulus_log.0);
    let encoding_with_padding = get_encoding_with_padding(ciphertext_modulus);
    let glwe_dimension = params.glwe_dimension;
    let polynomial_size = params.polynomial_size;

    let mut rsc = TestResources::new();

    let f = |x: Scalar| x;

    let delta: Scalar = encoding_with_padding / msg_modulus;
    let mut msg = msg_modulus;

    let accumulator = generate_accumulator(
        polynomial_size,
        glwe_dimension.to_glwe_size(),
        msg_modulus.cast_into(),
        ciphertext_modulus,
        delta,
        f,
    );

    assert!(check_encrypted_content_respects_mod(
        &accumulator,
        ciphertext_modulus
    ));

    while msg != Scalar::ZERO {
        msg = msg.wrapping_sub(Scalar::ONE);

        let mut keys_gen = |params| generate_keys(params, &mut rsc);
        let keys = gen_keys_or_get_from_cache_if_enabled(params, &mut keys_gen);
        let (input_lwe_secret_key, output_lwe_secret_key, fbsk) =
            (keys.small_lwe_sk, keys.big_lwe_sk, keys.fbsk);

        for _ in 0..NB_TESTS {
            let plaintext = Plaintext(msg * delta);

            let lwe_ciphertext_in = allocate_and_encrypt_new_lwe_ciphertext(
                &input_lwe_secret_key,
                plaintext,
                lwe_modular_std_dev,
                ciphertext_modulus,
                &mut rsc.encryption_random_generator,
            );

            assert!(check_encrypted_content_respects_mod(
                &lwe_ciphertext_in,
                ciphertext_modulus
            ));

            let mut out_pbs_ct = LweCiphertext::new(
                Scalar::ZERO,
                output_lwe_secret_key.lwe_dimension().to_lwe_size(),
                ciphertext_modulus,
            );

            programmable_bootstrap_lwe_ciphertext(
                &lwe_ciphertext_in,
                &mut out_pbs_ct,
                &accumulator,
                &fbsk,
            );

            assert!(check_encrypted_content_respects_mod(
                &out_pbs_ct,
                ciphertext_modulus
            ));

            let decrypted = decrypt_lwe_ciphertext(&output_lwe_secret_key, &out_pbs_ct);

            let decoded = round_decode(decrypted.0, delta) % msg_modulus;

            assert_eq!(decoded, f(msg));
        }

        // In coverage, we break after one while loop iteration, changing message values does not
        // yield higher coverage
        #[cfg(feature = "__coverage")]
        break;
    }
}

create_parametrized_test!(lwe_encrypt_pbs_decrypt_custom_mod {
    TEST_PARAMS_4_BITS_NATIVE_U64,
    TEST_PARAMS_3_BITS_63_U64
});

// DISCLAIMER: all parameters here are not guaranteed to be secure or yield correct computations
pub const TEST_PARAMS_4_BITS_NATIVE_U128: ClassicTestParams<u128> = ClassicTestParams {
    lwe_dimension: LweDimension(742),
    glwe_dimension: GlweDimension(1),
    polynomial_size: PolynomialSize(2048),
    lwe_modular_std_dev: StandardDev(4.9982771e-11),
    glwe_modular_std_dev: StandardDev(8.6457178e-32),
    pbs_base_log: DecompositionBaseLog(23),
    pbs_level: DecompositionLevelCount(1),
    ks_level: DecompositionLevelCount(5),
    ks_base_log: DecompositionBaseLog(3),
    pfks_level: DecompositionLevelCount(1),
    pfks_base_log: DecompositionBaseLog(23),
    pfks_modular_std_dev: StandardDev(0.00000000000000029403601535432533),
    cbs_level: DecompositionLevelCount(0),
    cbs_base_log: DecompositionBaseLog(0),
    message_modulus_log: CiphertextModulusLog(4),
    ciphertext_modulus: CiphertextModulus::new_native(),
};

pub const TEST_PARAMS_3_BITS_127_U128: ClassicTestParams<u128> = ClassicTestParams {
    lwe_dimension: LweDimension(742),
    glwe_dimension: GlweDimension(1),
    polynomial_size: PolynomialSize(2048),
    lwe_modular_std_dev: StandardDev(4.9982771e-11),
    glwe_modular_std_dev: StandardDev(8.6457178e-32),
    pbs_base_log: DecompositionBaseLog(23),
    pbs_level: DecompositionLevelCount(1),
    ks_level: DecompositionLevelCount(5),
    ks_base_log: DecompositionBaseLog(3),
    pfks_level: DecompositionLevelCount(1),
    pfks_base_log: DecompositionBaseLog(23),
    pfks_modular_std_dev: StandardDev(0.00000000000000029403601535432533),
    cbs_level: DecompositionLevelCount(0),
    cbs_base_log: DecompositionBaseLog(0),
    message_modulus_log: CiphertextModulusLog(3),
    ciphertext_modulus: CiphertextModulus::new(1 << 127),
};

fn lwe_encrypt_pbs_f128_decrypt_custom_mod<Scalar>(params: ClassicTestParams<Scalar>)
where
    Scalar: UnsignedTorus
        + Sync
        + Send
        + CastFrom<usize>
        + CastInto<usize>
        + Serialize
        + DeserializeOwned,
    ClassicTestParams<Scalar>: KeyCacheAccess<Keys = ClassicBootstrapKeys<Scalar>>,
{
    let input_lwe_dimension = params.lwe_dimension;
    let lwe_modular_std_dev = params.lwe_modular_std_dev;
    let ciphertext_modulus = params.ciphertext_modulus;
    let message_modulus_log = params.message_modulus_log;
    let msg_modulus = Scalar::ONE.shl(message_modulus_log.0);
    let encoding_with_padding = get_encoding_with_padding(ciphertext_modulus);
    let glwe_dimension = params.glwe_dimension;
    let polynomial_size = params.polynomial_size;
    let decomp_base_log = params.pbs_base_log;
    let decomp_level_count = params.pbs_level;

    let mut rsc = TestResources::new();

    let f = |x: Scalar| x;

    let delta: Scalar = encoding_with_padding / msg_modulus;
    let mut msg = msg_modulus;

    let accumulator = generate_accumulator(
        polynomial_size,
        glwe_dimension.to_glwe_size(),
        msg_modulus.cast_into(),
        ciphertext_modulus,
        delta,
        f,
    );

    assert!(check_encrypted_content_respects_mod(
        &accumulator,
        ciphertext_modulus
    ));

    while msg != Scalar::ZERO {
        msg = msg.wrapping_sub(Scalar::ONE);

        let mut keys_gen = |params| generate_keys(params, &mut rsc);

        let keys = gen_keys_or_get_from_cache_if_enabled(params, &mut keys_gen);
        let (input_lwe_secret_key, output_lwe_secret_key, bsk) =
            (keys.small_lwe_sk, keys.big_lwe_sk, keys.bsk);

        let mut fbsk = Fourier128LweBootstrapKey::new(
            input_lwe_dimension,
            glwe_dimension.to_glwe_size(),
            polynomial_size,
            decomp_base_log,
            decomp_level_count,
        );

        convert_standard_lwe_bootstrap_key_to_fourier_128(&bsk, &mut fbsk);

        drop(bsk);

        for _ in 0..NB_TESTS {
            let plaintext = Plaintext(msg * delta);

            let lwe_ciphertext_in = allocate_and_encrypt_new_lwe_ciphertext(
                &input_lwe_secret_key,
                plaintext,
                lwe_modular_std_dev,
                ciphertext_modulus,
                &mut rsc.encryption_random_generator,
            );

            assert!(check_encrypted_content_respects_mod(
                &lwe_ciphertext_in,
                ciphertext_modulus
            ));

            let mut out_pbs_ct = LweCiphertext::new(
                Scalar::ZERO,
                output_lwe_secret_key.lwe_dimension().to_lwe_size(),
                ciphertext_modulus,
            );

            programmable_bootstrap_f128_lwe_ciphertext(
                &lwe_ciphertext_in,
                &mut out_pbs_ct,
                &accumulator,
                &fbsk,
            );

            assert!(check_encrypted_content_respects_mod(
                &out_pbs_ct,
                ciphertext_modulus
            ));

            let decrypted = decrypt_lwe_ciphertext(&output_lwe_secret_key, &out_pbs_ct);

            let decoded = round_decode(decrypted.0, delta) % msg_modulus;

            assert_eq!(decoded, f(msg));
        }

        // In coverage, we break after one while loop iteration, changing message values does not
        // yield higher coverage
        #[cfg(feature = "__coverage")]
        break;
    }
}

#[test]
fn lwe_encrypt_pbs_f128_decrypt_custom_mod_test_params_4_bits_native_u128() {
    lwe_encrypt_pbs_f128_decrypt_custom_mod(TEST_PARAMS_4_BITS_NATIVE_U128);
}
#[test]
fn lwe_encrypt_pbs_f128_decrypt_custom_mod_test_params_3_bits_127_u128() {
    lwe_encrypt_pbs_f128_decrypt_custom_mod(TEST_PARAMS_3_BITS_127_U128);
}

#[derive(Clone, Copy)]
pub struct TestParams<Scalar: UnsignedInteger> {
    pub lwe_dimension: LweDimension,
    pub glwe_dimension: GlweDimension,
    pub polynomial_size: PolynomialSize,
    pub lwe_modular_std_dev: StandardDev,
    pub glwe_modular_std_dev: StandardDev,
    pub pbs_base_log: DecompositionBaseLog,
    pub pbs_level: DecompositionLevelCount,
    pub ks_level: DecompositionLevelCount,
    pub ks_base_log: DecompositionBaseLog,
    pub pfks_level: DecompositionLevelCount,
    pub pfks_base_log: DecompositionBaseLog,
    pub pfks_modular_std_dev: StandardDev,
    pub cbs_level: DecompositionLevelCount,
    pub cbs_base_log: DecompositionBaseLog,
    pub message_modulus_log: CiphertextModulusLog,
    pub ciphertext_modulus: CiphertextModulus<Scalar>,
}

fn lwe_encrypt_ntt_pbs_decrypt_custom_mod(params: TestParams<u64>) {
    let input_lwe_dimension = params.lwe_dimension;
    let lwe_modular_std_dev = params.lwe_modular_std_dev;
    let glwe_modular_std_dev = params.glwe_modular_std_dev;
    let ciphertext_modulus = params.ciphertext_modulus;
    let message_modulus_log = params.message_modulus_log;
    let msg_modulus = 1u64 << message_modulus_log.0;
    let encoding_with_padding = get_encoding_with_padding(ciphertext_modulus);
    let glwe_dimension = params.glwe_dimension;
    let polynomial_size = params.polynomial_size;
    let decomp_base_log = params.pbs_base_log;
    let decomp_level_count = params.pbs_level;
    let mut rsc = TestResources::new();

    let f = |x: u64| x.wrapping_rem(msg_modulus);

    let delta: u64 = encoding_with_padding / msg_modulus;
    let mut msg = msg_modulus;
    const NB_TESTS: usize = 10;

    let accumulator = generate_accumulator(
        polynomial_size,
        glwe_dimension.to_glwe_size(),
        msg_modulus.cast_into(),
        ciphertext_modulus,
        delta,
        f,
    );

    assert!(check_encrypted_content_respects_mod(
        &accumulator,
        ciphertext_modulus
    ));

    // Create the LweSecretKey
    let input_lwe_secret_key = allocate_and_generate_new_binary_lwe_secret_key(
        input_lwe_dimension,
        &mut rsc.secret_random_generator,
    );
    let output_glwe_secret_key = allocate_and_generate_new_binary_glwe_secret_key(
        glwe_dimension,
        polynomial_size,
        &mut rsc.secret_random_generator,
    );
    let output_lwe_secret_key = output_glwe_secret_key.clone().into_lwe_secret_key();

    let mut bsk = LweBootstrapKey::new(
        0u64,
        glwe_dimension.to_glwe_size(),
        polynomial_size,
        decomp_base_log,
        decomp_level_count,
        input_lwe_dimension,
        ciphertext_modulus,
    );

    par_generate_lwe_bootstrap_key(
        &input_lwe_secret_key,
        &output_glwe_secret_key,
        &mut bsk,
        glwe_modular_std_dev,
        &mut rsc.encryption_random_generator,
    );

    assert!(check_encrypted_content_respects_mod(
        &*bsk,
        ciphertext_modulus
    ));

    use crate::core_crypto::ntt_impl::ntt64::crypto::bootstrap::{
        bootstrap_scratch, NttLweBootstrapKeyOwned,
    };
    use crate::core_crypto::ntt_impl::ntt64::math::ntt::Ntt;

    let mut nbsk = NttLweBootstrapKeyOwned::new(
        bsk.input_lwe_dimension(),
        bsk.glwe_size(),
        bsk.polynomial_size(),
        bsk.decomposition_base_log(),
        bsk.decomposition_level_count(),
    );

    let mut buffers = ComputationBuffers::new();

    let ntt = Ntt::new(ciphertext_modulus, nbsk.polynomial_size());
    let ntt = ntt.as_view();

    let stack_size = bootstrap_scratch(glwe_dimension.to_glwe_size(), polynomial_size, ntt)
        .unwrap()
        .try_unaligned_bytes_required()
        .unwrap();

    buffers.resize(stack_size);

    nbsk.as_mut_view().fill_with_forward_ntt(bsk.as_view(), ntt);

    drop(bsk);

    while msg != 0u64 {
        msg = msg.wrapping_sub(1u64);
        for _ in 0..NB_TESTS {
            let plaintext = Plaintext(msg * delta);

            let lwe_ciphertext_in = allocate_and_encrypt_new_lwe_ciphertext(
                &input_lwe_secret_key,
                plaintext,
                lwe_modular_std_dev,
                ciphertext_modulus,
                &mut rsc.encryption_random_generator,
            );

            assert!(check_encrypted_content_respects_mod(
                &lwe_ciphertext_in,
                ciphertext_modulus
            ));

            let mut out_pbs_ct = LweCiphertext::new(
                0u64,
                output_lwe_secret_key.lwe_dimension().to_lwe_size(),
                ciphertext_modulus,
            );

            nbsk.as_view().bootstrap(
                out_pbs_ct.as_mut_view(),
                lwe_ciphertext_in.as_view(),
                accumulator.as_view(),
                ntt,
                buffers.stack(),
            );

            assert!(check_encrypted_content_respects_mod(
                &out_pbs_ct,
                ciphertext_modulus
            ));

            let decrypted = decrypt_lwe_ciphertext(&output_lwe_secret_key, &out_pbs_ct);

            let decoded = round_decode(decrypted.0, delta) % msg_modulus;

            assert_eq!(decoded, f(msg));
        }
    }
}

// Tweaked test params to run NTT PBS with original noise of 2_2 param set.
pub const TEST_PARAMS_3_BITS_SOLINAS_NTT_U64: TestParams<u64> = TestParams {
    lwe_dimension: LweDimension(742),
    glwe_dimension: GlweDimension(1),
    polynomial_size: PolynomialSize(2048),
    lwe_modular_std_dev: StandardDev(0.000007069849454709433),
    glwe_modular_std_dev: StandardDev(0.00000000000000029403601535432533),
    pbs_base_log: DecompositionBaseLog(12),
    pbs_level: DecompositionLevelCount(2),
    ks_level: DecompositionLevelCount(5),
    ks_base_log: DecompositionBaseLog(3),
    pfks_level: DecompositionLevelCount(1),
    pfks_base_log: DecompositionBaseLog(23),
    pfks_modular_std_dev: StandardDev(0.00000000000000029403601535432533),
    cbs_level: DecompositionLevelCount(0),
    cbs_base_log: DecompositionBaseLog(0),
    message_modulus_log: CiphertextModulusLog(4),
    ciphertext_modulus: CiphertextModulus::new((1 << 64) - (1 << 32) + 1),
};

pub const TEST_PARAMS_2_BITS_MICKAEL_PRIME_U64: TestParams<u64> = TestParams {
    lwe_dimension: LweDimension(568),
    glwe_dimension: GlweDimension(2),
    polynomial_size: PolynomialSize(512),
    lwe_modular_std_dev: StandardDev(0.000007069849454709433),
    glwe_modular_std_dev: StandardDev(0.00000000000000029403601535432533),
    pbs_base_log: DecompositionBaseLog(8),
    pbs_level: DecompositionLevelCount(2),
    ks_level: DecompositionLevelCount(2),
    ks_base_log: DecompositionBaseLog(8),
    pfks_level: DecompositionLevelCount(1),
    pfks_base_log: DecompositionBaseLog(23),
    pfks_modular_std_dev: StandardDev(0.00000000000000029403601535432533),
    cbs_level: DecompositionLevelCount(0),
    cbs_base_log: DecompositionBaseLog(0),
    message_modulus_log: CiphertextModulusLog(1),
    ciphertext_modulus: CiphertextModulus::new(4294828033),
};

create_parametrized_test!(lwe_encrypt_ntt_pbs_decrypt_custom_mod {
    TEST_PARAMS_3_BITS_SOLINAS_NTT_U64,
    TEST_PARAMS_2_BITS_MICKAEL_PRIME_U64
});

// norm2 is the value to multiply the ciphertext by to check noise is fine
fn lwe_encrypt_atomic_pattern_ks_ntt_pbs_decrypt_custom_mod(params: TestParams<u64>, norm2: u64) {
    let input_lwe_dimension = params.lwe_dimension;
    let lwe_modular_std_dev = params.lwe_modular_std_dev;
    let glwe_modular_std_dev = params.glwe_modular_std_dev;
    let ciphertext_modulus = params.ciphertext_modulus;
    let message_modulus_log = params.message_modulus_log;
    let msg_modulus = 1u64 << message_modulus_log.0;
    let encoding_with_padding = get_encoding_with_padding(ciphertext_modulus);
    let glwe_dimension = params.glwe_dimension;
    let polynomial_size = params.polynomial_size;
    let decomp_base_log = params.pbs_base_log;
    let decomp_level_count = params.pbs_level;
    let ks_decomp_base_log = params.ks_base_log;
    let ks_decomp_level_count = params.ks_level;
    let mut rsc = TestResources::new();

    let f = |x: u64| x.wrapping_rem(msg_modulus);

    let delta: u64 = encoding_with_padding / msg_modulus;
    let mut msg = msg_modulus;
    const NB_TESTS: usize = 10;

    let accumulator = generate_accumulator(
        polynomial_size,
        glwe_dimension.to_glwe_size(),
        msg_modulus.cast_into(),
        ciphertext_modulus,
        delta,
        f,
    );

    assert!(check_encrypted_content_respects_mod(
        &accumulator,
        ciphertext_modulus
    ));

    // Create the LweSecretKey
    let input_lwe_secret_key = allocate_and_generate_new_binary_lwe_secret_key(
        input_lwe_dimension,
        &mut rsc.secret_random_generator,
    );
    let output_glwe_secret_key = allocate_and_generate_new_binary_glwe_secret_key(
        glwe_dimension,
        polynomial_size,
        &mut rsc.secret_random_generator,
    );
    let output_lwe_secret_key = output_glwe_secret_key.clone().into_lwe_secret_key();

    let ksk_big_to_small = allocate_and_generate_new_lwe_keyswitch_key(
        &output_lwe_secret_key,
        &input_lwe_secret_key,
        ks_decomp_base_log,
        ks_decomp_level_count,
        lwe_modular_std_dev,
        ciphertext_modulus,
        &mut rsc.encryption_random_generator,
    );

    let mut bsk = LweBootstrapKey::new(
        0u64,
        glwe_dimension.to_glwe_size(),
        polynomial_size,
        decomp_base_log,
        decomp_level_count,
        input_lwe_dimension,
        ciphertext_modulus,
    );

    par_generate_lwe_bootstrap_key(
        &input_lwe_secret_key,
        &output_glwe_secret_key,
        &mut bsk,
        glwe_modular_std_dev,
        &mut rsc.encryption_random_generator,
    );

    assert!(check_encrypted_content_respects_mod(
        &*bsk,
        ciphertext_modulus
    ));

    use crate::core_crypto::ntt_impl::ntt64::crypto::bootstrap::{
        bootstrap_scratch, NttLweBootstrapKeyOwned,
    };
    use crate::core_crypto::ntt_impl::ntt64::math::ntt::Ntt;

    let mut nbsk = NttLweBootstrapKeyOwned::new(
        bsk.input_lwe_dimension(),
        bsk.glwe_size(),
        bsk.polynomial_size(),
        bsk.decomposition_base_log(),
        bsk.decomposition_level_count(),
    );

    let mut buffers = ComputationBuffers::new();

    let ntt = Ntt::new(ciphertext_modulus, nbsk.polynomial_size());
    let ntt = ntt.as_view();

    let stack_size = bootstrap_scratch(glwe_dimension.to_glwe_size(), polynomial_size, ntt)
        .unwrap()
        .try_unaligned_bytes_required()
        .unwrap();

    buffers.resize(stack_size);

    nbsk.as_mut_view().fill_with_forward_ntt(bsk.as_view(), ntt);

    drop(bsk);

    while msg != 0u64 {
        msg = msg.wrapping_sub(1u64);

        let plaintext = Plaintext(msg * delta);
        let mut lwe_ciphertext_in_and_pbs_out = allocate_and_encrypt_new_lwe_ciphertext(
            &output_lwe_secret_key,
            plaintext,
            glwe_modular_std_dev,
            ciphertext_modulus,
            &mut rsc.encryption_random_generator,
        );

        let mut out_ks_ct =
            LweCiphertext::new(0u64, ksk_big_to_small.output_lwe_size(), ciphertext_modulus);

        for _ in 0..NB_TESTS {
            // Subtract the plaintext to have a 0
            lwe_ciphertext_plaintext_sub_assign(&mut lwe_ciphertext_in_and_pbs_out, plaintext);

            assert!(check_encrypted_content_respects_mod(
                &lwe_ciphertext_in_and_pbs_out,
                ciphertext_modulus
            ));

            // Do the cleartext multiplication to test norm2
            lwe_ciphertext_cleartext_mul_assign_other_mod(
                &mut lwe_ciphertext_in_and_pbs_out,
                Cleartext(norm2),
            );

            assert!(check_encrypted_content_respects_mod(
                &lwe_ciphertext_in_and_pbs_out,
                ciphertext_modulus
            ));

            // Add back the plaintext to have the noisy version
            lwe_ciphertext_plaintext_add_assign(&mut lwe_ciphertext_in_and_pbs_out, plaintext);

            assert!(check_encrypted_content_respects_mod(
                &lwe_ciphertext_in_and_pbs_out,
                ciphertext_modulus
            ));

            keyswitch_lwe_ciphertext(
                &ksk_big_to_small,
                &lwe_ciphertext_in_and_pbs_out,
                &mut out_ks_ct,
            );

            nbsk.as_view().bootstrap(
                lwe_ciphertext_in_and_pbs_out.as_mut_view(),
                out_ks_ct.as_view(),
                accumulator.as_view(),
                ntt,
                buffers.stack(),
            );

            assert!(check_encrypted_content_respects_mod(
                &lwe_ciphertext_in_and_pbs_out,
                ciphertext_modulus
            ));

            let decrypted =
                decrypt_lwe_ciphertext(&output_lwe_secret_key, &lwe_ciphertext_in_and_pbs_out);

            let decoded = round_decode(decrypted.0, delta) % msg_modulus;

            assert_eq!(decoded, f(msg));
        }
    }
}

#[test]
fn lwe_encrypt_atomic_pattern_ks_ntt_pbs_decrypt_custom_mod_test_params_3_bits_solinas_ntt_u64() {
    lwe_encrypt_atomic_pattern_ks_ntt_pbs_decrypt_custom_mod(TEST_PARAMS_3_BITS_SOLINAS_NTT_U64, 2)
}
#[test]
fn lwe_encrypt_atomic_pattern_ks_ntt_pbs_decrypt_custom_mod_test_params_2_bits_mickael_prime_u64() {
    lwe_encrypt_atomic_pattern_ks_ntt_pbs_decrypt_custom_mod(
        TEST_PARAMS_2_BITS_MICKAEL_PRIME_U64,
        2,
    )
}
