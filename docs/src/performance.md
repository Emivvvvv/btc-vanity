# Performance and benchmarking

There is no universal fastest backend or candidate rate. Results vary with the
chain pipeline, match mode, case policy, pattern length, CPU, GPU, driver,
graphics API, power mode, temperature, compiler, and whether GPU state is warm.

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

The repository's Criterion benchmark separates cold GPU initialization from
fixed-work CPU and GPU chain measurements. Exact commands and contributor
verification expectations live in `CONTRIBUTING.md`; keep generated benchmark
artifacts out of secret-bearing output locations.

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
