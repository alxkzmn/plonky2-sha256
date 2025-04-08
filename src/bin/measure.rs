#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use jemalloc_ctl::{
    epoch,
    stats::{self},
};
use plonky2::{plonk::config::PoseidonGoldilocksConfig, util::serialization::Write};
use plonky2_sha256::bench::{prove, sha256_no_lookup_prepare};
use plonky2_u32::gates::arithmetic_u32::{U32GateSerializer, U32GeneratorSerializer};

const D: usize = 2;
type C = PoseidonGoldilocksConfig;

fn main() {
    epoch::advance().unwrap();
    //let allocated_before = stats::allocated::read().unwrap();
    let resident_before = stats::resident::read().unwrap();

    let (data, pw) = sha256_no_lookup_prepare();

    epoch::advance().unwrap();
    //let allocated_after = stats::allocated::read().unwrap();
    let resident_after = stats::resident::read().unwrap();
    println!(
        "Preprocessing: resident memory: {}GB",
        //(allocated_after - allocated_before) as f32 / 1024.0 / 1024.0 / 1024.0,
        (resident_after - resident_before) as f32 / 1024.0 / 1024.0 / 1024.0,
    );

    let gate_serializer = U32GateSerializer;
    let common_data_size = data.common.to_bytes(&gate_serializer).unwrap().len();
    let generator_serializer = U32GeneratorSerializer::<C, D>::default();
    let prover_data_size = data
        .prover_only
        .to_bytes(&generator_serializer, &data.common)
        .unwrap()
        .len();

    println!(
        "Common data size: {}B, Prover data size: {}B",
        common_data_size, prover_data_size
    );
    epoch::advance().unwrap();
    //let allocated_before = stats::allocated::read().unwrap();
    let resident_before = stats::resident::read().unwrap();

    let proof = prove(&data.prover_data(), pw);
    epoch::advance().unwrap();
    //let allocated_after = stats::allocated::read().unwrap();
    let resident_after = stats::resident::read().unwrap();
    println!(
        "Proving: resident memory: {}GB",
        // Allocation numbers for prover don't make sense (~17179870000GB?)
        //(allocated_after - allocated_before) as f32 / 1024.0 / 1024.0 / 1024.0,
        (resident_after - resident_before) as f32 / 1024.0 / 1024.0 / 1024.0,
    );

    let mut buffer = Vec::new();
    buffer.write_proof(&proof.proof).unwrap();
    println!("Proof size: {}B", buffer.len());
}
