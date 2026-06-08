"""Cumulative GC skew and origin heuristic (reference)."""


def min_skew_position(seq: str) -> int:
    seq = seq.upper()
    skew = 0
    best_pos, best_val = 0, 0
    for i, base in enumerate(seq, start=1):
        if base == "G":
            skew += 1
        elif base == "C":
            skew -= 1
        if skew < best_val:
            best_val, best_pos = skew, i
    return best_pos


def main():
    pos = min_skew_position("CCTATCGGTACCGGAA")
    assert isinstance(pos, int) and pos >= 0
    print(f"min GC-skew at position {pos}")


if __name__ == "__main__":
    main()
