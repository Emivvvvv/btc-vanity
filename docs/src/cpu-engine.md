# CPU engine

The CPU engine is the compatibility baseline and the only regular-expression
engine.

## Workers and batches

The engine creates the requested number of worker threads. Each worker:

1. generates an initial array of 256 chain keypairs;
2. scans that batch for a match;
3. refills the existing array with new candidates; and
4. repeats until a shared atomic stop flag is set.

Exact-match scans visit candidates in small unrolled groups. They check the stop
flag periodically rather than on every comparison, reducing synchronization
overhead. When a worker wins, it swaps the matching value out of the array,
claims the atomic flag, and sends the owned keypair to the waiting thread.

Batching reuses storage and keeps generation and matching work local to each
worker. It does not mean candidates are written to disk or exposed through the
batch-file interface.

## Compiled exact matching

The exact pattern is compiled once per search and shared by all workers:

- case-sensitive prefix, suffix, and substring paths use byte-oriented memory
  comparison;
- case-insensitive patterns are lowercased once through an ASCII lookup table;
- prefix and suffix compare only the relevant address slice;
- one-character substring search uses a direct scan;
- medium case-insensitive substrings use a precomputed bad-character table; and
- other substring lengths use a straightforward bounded scan.

Preparing this state once avoids lowercasing or rebuilding search tables for
every candidate.

## Regex matching

Regex syntax is validated and compiled before useful work begins. Each regex
worker generates the same 256-candidate batches and applies the compiled
expression to address text. Regex has its own first-winner atomic and channel.

Regex is more flexible but cannot use the compiled exact matcher or GPU shader
path. Prefer prefix, suffix, or anywhere when those modes express the goal.

## Thread count

The CLI defaults to the machine's reported available parallelism. More threads
can increase throughput until curve, hashing, memory, thermal, or scheduling
limits dominate. It can also interfere with other workloads. Measure several
worker counts on the target machine rather than assuming all logical CPUs are
best.
