# Reproducibility Benchmark Harness

Phase 5 of the [superiority strategy](../../STRATEGY.md). Turns "Dirac-Paper2Codes
is superior" into a measurement by running a curated set of paper→repo cases and
scoring the generated repositories.

## Scoring

- **Reference-based** (needs an author-released repo): file-set precision /
  recall / **F1** by normalised filename, plus token-Jaccard **content
  similarity** over matched files. Headline `overall = 0.5·F1 + 0.5·content`.
- **Reference-free** (no ground truth): an optional LLM-judge rubric score in
  `[0, 1]`, wired via the `RubricGrader` trait.

Both feed an aggregate report (Markdown + JSON).

## Manifest

A JSON file describing the cases. Relative paths resolve against the manifest's
directory. See [`manifest.example.json`](manifest.example.json):

```json
{
  "name": "my-suite",
  "cases": [
    { "id": "resnet", "paper_path": "papers/resnet.pdf", "reference_repo": "refs/resnet" }
  ]
}
```

`reference_repo` and `rubric_path` are optional per case.

## Running

```bash
# Requires LLM API keys configured (see ../../LLM_API_KEY.md)
paper2codes bench --manifest bench/manifest.example.json --output report.json
```

The CLI runs the full paper→repository pipeline per case and prints a Markdown
report; `--output` also writes the JSON report. The CLI performs reference-based
scoring; reference-free rubric grading is available programmatically through the
`paper2codes::benchmark` API (`RubricGrader`).

## Library API

```rust
use paper2codes::benchmark::{run_benchmark, BenchmarkManifest, RepoGenerator};

let manifest = BenchmarkManifest::load("bench/manifest.json")?;
let report = run_benchmark(&manifest, &my_generator, Some(&my_grader)).await;
println!("{}", report.to_markdown());
```

`RepoGenerator` and `RubricGrader` are traits, so the pipeline is fully
unit-tested with mocks and the production path wraps the real coordinator.
