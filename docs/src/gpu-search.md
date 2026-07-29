# GPU and Hybrid search

The experimental GPU backend runs candidate derivation, chain-specific hashing,
address encoding, and exact matching in compute shaders. wgpu selects a native
Metal, Vulkan, or DirectX 12 path according to the platform and driver.

```bash
btc-vanity --backend hybrid --gpu-usage-limit 70 abc
btc-vanity --backend gpu --gpu-usage-limit 100 abc
```

Hybrid defaults to a 70% usage limit. Explicit GPU defaults to 100%. The limit
is a best-effort dispatch duty cycle, not a utilization target, power cap,
temperature limit, or scheduler guarantee.

## Responsiveness and batch latency

At 100%, the engine favors throughput by keeping multiple dispatch slots in
flight. Large batches reduce scheduling overhead, but a dispatch that is
already running cannot be divided by the application. This can delay display
work and increases the time before a CPU winner can stop a Hybrid GPU worker.

Below 100%, the engine:

- caps an individual dispatch at 4,096 candidates;
- keeps one slot in flight;
- waits for the dispatch to complete; and
- sleeps in proportion to the active time.

This produces more scheduling opportunities for other graphics work, but it
also lowers throughput and still may not eliminate stutter on every driver.

If the desktop becomes unresponsive:

1. lower `--gpu-usage-limit`, for example to 50;
2. reduce `--gpu-batch-size`;
3. choose Hybrid so CPU can also make progress; or
4. choose CPU to remove search work from the graphics queue.

## Batch-size control

The normal GPU tuning starts at 262,144 candidates with two dispatch slots.
`--gpu-batch-size N` overrides the requested batch size. Values above the
engine maximum are capped at 2,097,152. Short patterns and active usage
limiting may reduce the effective size further.

Larger is not universally better. It can improve steady-state throughput while
worsening time-to-first-result, cancellation latency, and display scheduling.
Benchmark a few sizes on the target system instead of assuming the largest
value wins.

## Supported matching and fallback

GPU shaders support prefix, suffix, and anywhere matching for Bitcoin,
Ethereum, and Solana. Regex is CPU-only. Explicit GPU plus regex returns an
error; Auto and Hybrid use CPU.

Hybrid starts cancelable CPU and GPU workers and returns the first valid
candidate. If GPU initialization or execution fails, CPU work remains
available. Explicit GPU reports the failure.

## Trust boundary

GPU search places seed material and candidate state in local graphics-device
buffers and passes through the system graphics stack. Use CPU when the device
or driver is outside the system boundary you are prepared to trust. Regardless
of backend, [verify and protect the result](security.md).
