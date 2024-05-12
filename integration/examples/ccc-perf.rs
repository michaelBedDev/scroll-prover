#![feature(core_intrinsics)]

use integration::test_util::{ccc_as_signer, prepare_circuit_capacity_checker};
use prover::BlockTrace;
use std::{
    fs::{read_to_string, File},
    intrinsics::black_box,
    mem,
};
use zkevm_circuits::witness::CHECK_VALUE_COST;

fn main() {
    prepare_circuit_capacity_checker();

    let path = std::env::var("TRACE_PATH").unwrap();

    let now = std::time::Instant::now();

    let traces = glob::glob(format!("{path}/**/*.json").as_str())
        .unwrap()
        .flatten()
        .map(|p| read_to_string(p))
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
        .into_iter()
        .map(|s| {
            serde_json::from_str(&s).or_else(|_| {
                #[derive(serde::Deserialize, Default, Debug, Clone)]
                pub struct BlockTraceJsonRpcResult {
                    pub result: BlockTrace,
                }
                serde_json::from_str::<BlockTraceJsonRpcResult>(&s).map(|r| r.result)
            })
        })
        .collect::<Result<Vec<BlockTrace>, _>>()
        .unwrap();

    let len = traces.len();

    let deserialize_elapsed = now.elapsed();
    println!("json deserialize_elapsed: {:.2}ms", deserialize_elapsed.as_millis() as f64 / len as f64);

    let now = std::time::Instant::now();
    let msg_pack_traces = traces.iter().map(|t| rmp_serde::to_vec(t))
        .collect::<Result<Vec<Vec<u8>>, _>>()
        .unwrap();
    let serialize_elapsed = now.elapsed();
    println!("msg_pack serialize_elapsed: {:.2}ms", serialize_elapsed.as_millis() as f64 / len as f64);

    let now = std::time::Instant::now();
    let traces = msg_pack_traces.iter().map(|t| rmp_serde::from_slice(t))
        .collect::<Result<Vec<BlockTrace>, _>>()
        .unwrap();
    let deserialize_elapsed = now.elapsed();
    println!("msg_pack deserialize_elapsed: {:.2}ms", deserialize_elapsed.as_millis() as f64 / len as f64);


    let traces = traces.into_iter();
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();

    let now = std::time::Instant::now();
    for trace in traces {
        let trace = [trace];
        let result = ccc_as_signer(0, &trace);
        black_box(result); // avoid optimization
        mem::forget(trace); // leak, avoid
    }
    let ccc_elapsed = now.elapsed();

    if let Ok(report) = guard.report().build() {
        let file = File::create("flamegraph.svg").unwrap();
        report.flamegraph(file).unwrap();
    };

    let check_value_cost = CHECK_VALUE_COST.lock().unwrap();

    let check_value_cost = check_value_cost
        .iter()
        .map(|d| d.as_millis() as f64)
        .sum::<f64>()
        / check_value_cost.len() as f64;

    println!(
        "ccc_elapsed: {:.2}ms, check_value_cost: {:.2}ms",
        ccc_elapsed.as_millis() as f64 / len as f64,
        check_value_cost
    );
}
