# Choosing a backend

Backend choice is a trade-off among compatibility, startup cost, throughput,
and desktop responsiveness.

| Backend | Prefer it when |
| --- | --- |
| CPU | You use regex, need maximum compatibility, or want to leave the GPU idle |
| Hybrid | You want CPU and GPU to race during an interactive exact-pattern search |
| GPU | You want the exact-pattern GPU path and can tolerate GPU startup and display load |
| Auto | You want runtime selection and CPU fallback |

GPU and Hybrid are experimental.

## CLI and library defaults differ

The command-line program defaults to **Hybrid**. Without the `gpu` Cargo
feature, that path runs on CPU. With the feature, CPU and GPU workers race.

`VanitySearchOptions::default()` uses **Auto**. Auto currently selects:

- CPU for regex or exact patterns of at most 2 adjusted characters;
- Hybrid for exact patterns of 3 or 4 adjusted characters;
- GPU for longer exact patterns; and
- CPU whenever the requested GPU path cannot be initialized.

For Bitcoin prefix search, the adjusted pattern includes the fixed leading
`1`, which contributes to this internal length decision.

## Explicitness and fallback

Use `--backend cpu` when GPU work is unwanted. Use `--backend gpu` when GPU
availability is a requirement: a build without GPU support, an unavailable
adapter, or a GPU regex request returns an error instead of silently using CPU.

Auto falls back to CPU after a GPU or Hybrid attempt fails. Hybrid already has
a CPU worker and continues on CPU if its GPU worker fails. Regex under Auto or
Hybrid is routed directly to CPU.

The alias `--backend both` means Hybrid.

## Threads and Hybrid

`--threads` controls CPU workers. It has no direct effect on a pure GPU search.
Hybrid may cap its CPU side at four workers when using a GPU-dominant batch
size, reducing competition for host resources. A smaller explicit GPU batch
can allow Hybrid to use the full requested CPU worker count.

## A practical sequence

1. Use CPU for a short test and for every regex.
2. Compare CPU and Hybrid on a representative exact pattern.
3. Try explicit GPU only when initialization cost is small relative to the
   expected search.
4. Lower the GPU usage limit if interactive graphics stutter.

Performance depends on the complete chain pipeline, not only elliptic-curve
arithmetic. Measure the address and match mode you will actually use.
