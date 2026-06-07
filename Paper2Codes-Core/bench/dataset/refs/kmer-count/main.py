"""k-mer counting for genomic sequences (reference implementation)."""

from collections import Counter
from typing import Dict


def count_kmers(seq: str, k: int) -> Dict[str, int]:
    """Return a mapping from each length-k substring of seq to its count."""
    if k <= 0:
        raise ValueError("k must be positive")
    n = len(seq)
    if k > n:
        return {}
    counts: Counter = Counter(seq[i : i + k] for i in range(n - k + 1))
    return dict(counts)


def main():
    seq = "AGAGAG"
    counts = count_kmers(seq, 2)
    # n - k + 1 = 5 total k-mers: AG x3, GA x2
    assert sum(counts.values()) == len(seq) - 2 + 1
    assert counts["AG"] == 3 and counts["GA"] == 2
    print(counts)


if __name__ == "__main__":
    main()
