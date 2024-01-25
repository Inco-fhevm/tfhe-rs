use mpi::traits::*;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::time::Instant;
use tfhe::shortint::parameters::PARAM_MESSAGE_2_CARRY_2_KS_PBS;
use tfhe::shortint::{gen_keys, Ciphertext, ServerKey};

const N: u64 = 10000;
fn main() {
    mpi();
}

fn local() {
    const N: u64 = 2000;

    let (cks, sks) = gen_keys(PARAM_MESSAGE_2_CARRY_2_KS_PBS);

    let mut inputs = vec![];

    for i in 0..N {
        let ct = cks.unchecked_encrypt(i % 16);

        inputs.push(ct);
    }

    let lookup_table = sks.generate_lookup_table(|x| (x + 1) % 16);

    let start = Instant::now();

    let _outputs: Vec<_> = inputs
        .iter()
        // .par_iter()
        .map(|ct| sks.apply_lookup_table(ct, &lookup_table))
        .collect();

    let duration = start.elapsed();

    let duration_sec = duration.as_secs_f32();

    println!("{N} PBS in {}s", duration_sec);
    println!("{} ms/PBS", duration_sec * 1000. / N as f32);
}

fn mpi() {
    let universe = mpi::initialize().unwrap();
    let world = universe.world();
    let size = world.size() as usize;
    let rank = world.rank();
    let root_rank = 0;

    let is_root = rank == root_rank;

    if is_root {
        local();
    }

    let root_process = world.process_at_rank(root_rank);

    let mut cks_opt = None;

    let mut sks_serialized = vec![];
    let mut sks_serialized_len = 0;

    if is_root {
        let (cks, sks) = gen_keys(PARAM_MESSAGE_2_CARRY_2_KS_PBS);

        cks_opt = Some(cks);

        sks_serialized = bincode::serialize(&sks).unwrap();
        sks_serialized_len = sks_serialized.len();
    }

    root_process.broadcast_into(&mut sks_serialized_len);

    if sks_serialized.is_empty() {
        sks_serialized = vec![0; sks_serialized_len];
    }

    root_process.broadcast_into(&mut sks_serialized);

    let sks: ServerKey = bincode::deserialize(&sks_serialized).unwrap();

    let lookup_table = sks.generate_lookup_table(|x| (x + 1) % 16);

    if is_root {
        let cks = cks_opt.as_ref().unwrap();

        let mut inputs = vec![];

        for i in 0..N {
            let ct = cks.unchecked_encrypt(i % 16);

            inputs.push(ct);
        }

        let start = Instant::now();
        let elements_per_node = N as usize / size as usize;

        for dest_rank in 1..size {
            let process = world.process_at_rank(dest_rank as i32);

            let inputs_chunk =
                &inputs[elements_per_node * dest_rank..elements_per_node * (dest_rank + 1)];

            let inputs_chunk_serialized = bincode::serialize(inputs_chunk).unwrap();

            process.send(&inputs_chunk_serialized);
        }

        let mut outputs: Vec<_> = inputs[0..elements_per_node]
            .iter()
            .map(|ct| sks.apply_lookup_table(ct, &lookup_table))
            .collect();

        for dest_rank in 1..size {
            let process = world.process_at_rank(dest_rank as i32);

            let (outputs_chunks_serialized, _status) = process.receive_vec();

            let outputs_chunk: Vec<Ciphertext> =
                bincode::deserialize(&outputs_chunks_serialized).unwrap();

            outputs.extend(outputs_chunk);
        }

        let duration = start.elapsed();

        let duration_sec = duration.as_secs_f32();

        println!("{N} PBS in {}s", duration_sec);
        println!("{} ms/PBS", duration_sec * 1000. / N as f32);

        for (i, ct) in outputs.iter().enumerate() {
            assert_eq!(cks.decrypt_message_and_carry(ct), (i as u64 + 1) % 16);
        }

        println!("All good");
    } else {
        let (inputs_chunks_serialized, _status) = root_process.receive_vec();

        let inputs_chunk: Vec<Ciphertext> =
            bincode::deserialize(&inputs_chunks_serialized).unwrap();

        let outputs_chunk: Vec<_> = inputs_chunk
            .iter()
            .map(|ct| sks.apply_lookup_table(ct, &lookup_table))
            .collect();

        let outputs_chunk_serialized = bincode::serialize(&outputs_chunk).unwrap();

        root_process.send(&outputs_chunk_serialized);
    }
}
