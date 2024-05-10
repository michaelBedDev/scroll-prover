#![feature(core_intrinsics)]

use std::fs::{File, read_to_string};
use std::intrinsics::black_box;
use std::mem;
use prover::BlockTrace;
use integration::test_util::{ccc_as_signer, prepare_circuit_capacity_checker};

fn main() {
    prepare_circuit_capacity_checker();

    let path = std::env::var("TRACE_PATH").unwrap();

    let now = std::time::Instant::now();

    let traces = glob::glob(format!("{path}/**/*.json").as_str()).unwrap()
        .flatten()
        .map(|p| {
            read_to_string(p)
        })
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
        .into_iter()
        .map(|s| serde_json::from_str(&s).or_else(|_| {
            #[derive(serde::Deserialize, Default, Debug, Clone)]
            pub struct BlockTraceJsonRpcResult {
                pub result: BlockTrace,
            }
            serde_json::from_str::<BlockTraceJsonRpcResult>(&s)
                .map(|r| r.result)
        }))
        .collect::<Result<Vec<BlockTrace>, _>>()
        .unwrap();

    let len = traces.len();

    let traces = traces.into_iter();

    let deserialize_elapsed = now.elapsed();

    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();

    for trace in traces {
        let trace = [trace];
        let result = ccc_as_signer(0, &trace);
        black_box(result); // avoid optimization
        mem::forget(trace); // leak, avoid
    }

    if let Ok(report) = guard.report().build() {
        let file = File::create("flamegraph.svg").unwrap();
        report.flamegraph(file).unwrap();
    };

    let total_elapsed = now.elapsed();
    println!(
        "deserialize_elapsed: {:.2}ms, total_elapsed: {:.2}ms",
        deserialize_elapsed.as_millis() as f64 / len as f64,
        total_elapsed.as_millis() as f64 / len as f64,
    );
}