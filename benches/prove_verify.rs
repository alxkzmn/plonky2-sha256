use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    iop::witness::{PartialWitness, WitnessWrite},
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, ProverCircuitData, VerifierCircuitData},
        config::{GenericConfig, PoseidonGoldilocksConfig},
        proof::ProofWithPublicInputs,
    },
    util::serialization::Write,
};
use plonky2_sha256::circuit::{array_to_bits, make_circuits};
use plonky2_u32::gates::arithmetic_u32::{U32GateSerializer, U32GeneratorSerializer};
use rand::Rng;
use sha2::{Digest, Sha256};

const D: usize = 2;
type C = PoseidonGoldilocksConfig;
type F = <C as GenericConfig<D>>::F;

fn sha256_no_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha256_no_lookup");

    let (data, _) = sha256_no_lookup_prepare();
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

    group.sample_size(10);

    group.bench_function("sha256_no_lookup_prove", |bench| {
        bench.iter_batched(
            || sha256_no_lookup_prepare(),
            |(data, pw)| {
                prove(&data.prover_data(), pw);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("sha256_no_lookup_verify", |bench| {
        bench.iter_batched(
            || {
                let (data, pw) = sha256_no_lookup_prepare();
                let verifier_data = data.verifier_data();
                let proof = prove(&data.prover_data(), pw);
                let mut buffer = Vec::new();
                buffer.write_proof(&proof.proof).unwrap();
                println!("Proof size: {}B", buffer.len());
                (proof, verifier_data)
            },
            |(proof, data)| {
                verify(&data, proof);
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn verify(data: &VerifierCircuitData<F, C, D>, proof: ProofWithPublicInputs<F, C, D>) {
    data.verify(proof).unwrap()
}

fn prove(
    data: &ProverCircuitData<F, C, D>,
    pw: PartialWitness<F>,
) -> ProofWithPublicInputs<GoldilocksField, C, D> {
    data.prove(pw).unwrap()
}

fn sha256_no_lookup_prepare() -> (CircuitData<F, C, D>, PartialWitness<F>) {
    const MSG_BYTE_SIZE: usize = 2048;
    let mut msg = vec![0; MSG_BYTE_SIZE];
    let mut rng = rand::thread_rng();
    for msg_bit in msg.iter_mut().take(MSG_BYTE_SIZE - 1) {
        *msg_bit = rng.gen_range(0..=1);
    }

    let mut hasher = Sha256::new();
    hasher.update(&msg);
    let hash = hasher.finalize();
    // println!("Hash: {:#04X}", hash);

    let msg_bits = array_to_bits(&msg);
    let len = msg.len() * 8;
    println!("block count: {}", (len + 65 + 511) / 512);
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;
    let mut builder = CircuitBuilder::<F, D>::new(CircuitConfig::standard_recursion_config());
    let targets = make_circuits(&mut builder, len as u64);
    let mut pw = PartialWitness::new();

    for (i, msg_bit) in msg_bits.iter().enumerate().take(len) {
        pw.set_bool_target(targets.message[i], *msg_bit).unwrap();
    }

    let expected_res = array_to_bits(hash.as_slice());
    for (i, expected_res_bit) in expected_res.iter().enumerate() {
        if *expected_res_bit {
            builder.assert_one(targets.digest[i].target);
        } else {
            builder.assert_zero(targets.digest[i].target);
        }
    }

    println!(
        "Constructing inner proof with {} gates",
        builder.num_gates()
    );
    (builder.build::<C>(), pw)
}

criterion_main!(sha256);
criterion_group!(sha256, sha256_no_lookup);
