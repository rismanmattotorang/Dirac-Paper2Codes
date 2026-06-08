"""Hamming distance between equal-length DNA sequences (reference)."""

_ALPHABET = set("ACGT")


def hamming(a: str, b: str) -> int:
    a, b = a.upper(), b.upper()
    if len(a) != len(b):
        raise ValueError("sequences must be equal length")
    if (set(a) | set(b)) - _ALPHABET:
        raise ValueError("invalid nucleotide")
    return sum(1 for x, y in zip(a, b) if x != y)


def main():
    assert hamming("GAGCCTACTAACGGGAT", "CATCGTAATGACGGCCT") == 7
    assert hamming("ACGT", "ACGT") == 0
    print("hamming OK")


if __name__ == "__main__":
    main()
