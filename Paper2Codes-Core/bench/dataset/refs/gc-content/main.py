"""GC content and reverse complement of DNA sequences (reference)."""

_COMPLEMENT = {"A": "T", "T": "A", "C": "G", "G": "C"}


def gc_content(seq: str) -> float:
    """Fraction of G/C bases in [0, 1]."""
    s = seq.upper()
    if not s:
        return 0.0
    _validate(s)
    gc = sum(1 for b in s if b in ("G", "C"))
    return gc / len(s)


def reverse_complement(seq: str) -> str:
    """Reverse complement of a DNA sequence (preserves nothing but A/C/G/T)."""
    s = seq.upper()
    _validate(s)
    return "".join(_COMPLEMENT[b] for b in reversed(s))


def _validate(s: str) -> None:
    invalid = set(s) - set(_COMPLEMENT)
    if invalid:
        raise ValueError(f"invalid nucleotides: {sorted(invalid)}")


def main():
    seq = "GATTACA"
    rc = reverse_complement(seq)
    assert reverse_complement(rc) == seq  # involution
    print(f"GC({seq}) = {gc_content(seq):.3f}, reverse_complement = {rc}")


if __name__ == "__main__":
    main()
