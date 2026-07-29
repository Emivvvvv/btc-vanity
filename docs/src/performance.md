# Performance and benchmarking

btc-vanity is engineered to be blazingly fast across multithreaded CPU and
experimental GPU backends. However, there is no universal fastest backend or
candidate rate for every machine. Results vary with the chain pipeline, match
mode, case policy, pattern length, CPU, GPU, driver, graphics API, power mode,
temperature, compiler, and whether GPU state is warm.

## Measured results: Apple M1 Pro

On July 29, 2026, the version 3 branch was measured at tree
`db218489bd9d716814b3648496ffd9f285213bc2` on:

- Apple M1 Pro with 8 CPU cores reported to the process;
- 14-core integrated GPU;
- 16 GiB unified memory;
- macOS 15.7.4; and
- Metal 3.

The comparison used [vgen 0.3.0 at revision
`a405fcb69032d328318cfacbda9d7fea9d4dcacc`](https://github.com/oritwoen/vgen/tree/a405fcb69032d328318cfacbda9d7fea9d4dcacc).
Both CPU measurements performed a case-sensitive Bitcoin P2PKH candidate loop:
secp256k1 public-key derivation, compressed public-key serialization, address
generation, and prefix comparison. Measurements report Criterion medians
rather than the fastest sample.

| Tool and path | Fixed work and median | Derived throughput |
| --- | ---: | ---: |
| btc-vanity CPU, one benchmark worker | 304.019 ms / 16,384 candidates | **53,891 candidates/s** |
| vgen CPU, one benchmark worker | 21.432 µs / candidate | 46,659 candidates/s |
| btc-vanity Metal, batch 262,144 × ring depth 2 | 5.001 s / 524,288 candidates | **104,837 candidates/s** |

For this candidate loop, btc-vanity's one-worker CPU path was approximately
**15.5% faster than vgen's one-worker CPU path**. btc-vanity's Metal path
delivered approximately **1.95× the throughput of its own one-worker CPU
path**.

These are engine-level results, not a complete application leaderboard. The
study did not compare the tools' full multicore CPU schedulers, and it does not
show that Metal outperforms btc-vanity's full multithreaded CPU backend.

### Excluded comparisons

vgen's 262,144-candidate GPU benchmark could not be measured on this machine:
its Metal shader pipeline failed during compilation with an internal compiler
error. That failure is an excluded result, not evidence that btc-vanity is
faster.

Ethereum was excluded because vgen's CPU generator produced EIP-55 checksummed
addresses while btc-vanity searches lowercase 40-character hexadecimal
addresses, and vgen did not provide the corresponding GPU path. Solana was
excluded because the available `solana-keygen grind` command exposed neither a
fixed-work mode nor a throughput counter. Comparing random time-to-match runs
would mostly compare luck.

These figures should be replaced or expanded when equivalent full-pipeline,
multicore, and cross-platform measurements are available.

## Think in candidates and probability

Elapsed time combines two independent questions:

1. How many complete candidate addresses can this configuration test per
   second?
2. How many candidates does this pattern distribution require?

A high candidate rate cannot make an exponentially rarer target cheap. Report
both rate and pattern assumptions. Do not extrapolate a Base58 prefix from a
hexadecimal prefix or an anywhere match from a fixed-position match.

Search completion is random. A single successful run measures luck as well as
speed. Throughput benchmarks should perform a fixed amount of work; end-to-end
search studies should use many independent runs and report distributions.

## Cold and warm measurements

Separate:

- process startup;
- first GPU adapter/device creation;
- precomputed-table upload;
- shader rendering and pipeline compilation; and
- steady-state candidate batches using the cached engine.

Short searches may be dominated by cold GPU cost and can finish sooner on CPU
even when the GPU has higher warm throughput.

## Compare equivalent work

Use the same revision, release profile, feature set, chain, pattern bytes, case
setting, mode, and fixed candidate count. Verify that both paths derive the
same addresses from known private inputs and that every reported winner
reconstructs correctly.

For Hybrid, measure end-to-end latency as its own behavior. Adding standalone
CPU and GPU rates does not capture contention, cancellation, or the first-winner
race. Likewise, throttled and unthrottled GPU runs answer different operational
questions.

## A reproducible study

For every result, record:

- repository revision and whether the tree was clean;
- exact build and benchmark commands;
- Rust compiler and dependency lockfile;
- operating system, CPU, memory, GPU, driver, and wgpu backend;
- power source and power/performance mode;
- chain, mode, case policy, pattern length, worker count, GPU batch size, usage
  limit, and warm-up procedure;
- sample count and fixed work per sample;
- median and tail values, not only the best run;
- validation failures, adapter errors, and skipped samples; and
- whether the desktop was also driving displays or other GPU applications.

Run with a stable machine: close unrelated compute work, avoid thermal
throttling, and keep conditions identical between variants. Randomize or
alternate variant order when heat or boost behavior could bias later runs.

The repository's Criterion benchmark suite includes targets for CPU multithreaded scaling (`cpu_benchmarks`), pattern-matching algorithms (`pattern_matching`), and GPU candidate pipelines (`gpu_end_to_end`). Exact commands and contributor verification expectations live in `CONTRIBUTING.md`; keep generated benchmark artifacts out of secret-bearing output locations.

## Interpret results conservatively

Prefer qualitative conclusions tied to the measured configuration:

- CPU avoids GPU setup and display contention.
- GPU amortizes fixed setup better over larger workloads.
- Larger batches may improve throughput while increasing response latency.
- Lower usage limits trade throughput for more scheduling gaps.
- Hybrid may reduce time to the first result but consumes both host and device
  resources.

Do not present one machine's result as a product guarantee, and do not turn a
synthetic candidate loop into a claim about complete wallet-generation speed
unless the benchmark includes the full chain pipeline and matching work.
