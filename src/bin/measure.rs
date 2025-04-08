use mach2::{
    kern_return::KERN_SUCCESS, message::mach_msg_type_number_t, task::task_info,
    task_info::MACH_TASK_BASIC_INFO, traps::mach_task_self,
};
use plonky2::{
    plonk::config::{GenericConfig, PoseidonGoldilocksConfig},
    util::serialization::Write,
};
use plonky2_sha256::bench::{prove, sha256_no_lookup_prepare};
use plonky2_u32::gates::arithmetic_u32::{U32GateSerializer, U32GeneratorSerializer};

//https://github.com/apple/darwin-xnu/blob/main/osfmk/mach/task_info.h#L328
#[repr(C)]
#[derive(Debug, Default)]
struct MachTaskBasicInfo {
    virtual_size: u64,
    resident_size: u64,
    resident_size_max: u64,
    user_time: TimeValue,
    system_time: TimeValue,
    policy: i32,
    suspend_count: i32,
}
#[repr(C)]
#[derive(Debug, Default)]
struct TimeValue {
    seconds: i32,
    microseconds: i32,
}

fn get_rss_bytes() -> Option<usize> {
    unsafe {
        let mut info = MachTaskBasicInfo::default();
        let mut count = (std::mem::size_of::<MachTaskBasicInfo>() / std::mem::size_of::<u32>())
            as mach_msg_type_number_t;

        let result = task_info(
            mach_task_self(),
            MACH_TASK_BASIC_INFO,
            &mut info as *mut _ as *mut i32,
            &mut count,
        );

        if result == KERN_SUCCESS {
            Some(info.resident_size as usize)
        } else {
            None
        }
    }
}

const D: usize = 2;
type C = PoseidonGoldilocksConfig;

fn main() {
    let before = get_rss_bytes().unwrap();
    let (data, pw) = sha256_no_lookup_prepare();
    let after = get_rss_bytes().unwrap();
    println!(
        "Preprocessing ram usage: {} GB",
        (after - before) as f32 / 1024_f32 / 1024_f32 / 1024_f32
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
    let before = get_rss_bytes().unwrap();
    let proof = prove(&data.prover_data(), pw);
    let after = get_rss_bytes().unwrap();
    println!(
        "Prover ram usage: {} GB",
        (after - before) as f32 / 1024_f32 / 1024_f32 / 1024_f32
    );
    let mut buffer = Vec::new();
    buffer.write_proof(&proof.proof).unwrap();
    println!("Proof size: {}B", buffer.len());
}
