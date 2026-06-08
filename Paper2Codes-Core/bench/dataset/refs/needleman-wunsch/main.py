"""Needleman-Wunsch global alignment score (reference)."""


def nw_score(a: str, b: str, match: int = 1, mismatch: int = -1, gap: int = -1) -> int:
    m, n = len(a), len(b)
    f = [[0] * (n + 1) for _ in range(m + 1)]
    for i in range(m + 1):
        f[i][0] = i * gap
    for j in range(n + 1):
        f[0][j] = j * gap
    for i in range(1, m + 1):
        for j in range(1, n + 1):
            s = match if a[i - 1] == b[j - 1] else mismatch
            f[i][j] = max(f[i - 1][j - 1] + s, f[i - 1][j] + gap, f[i][j - 1] + gap)
    return f[m][n]


def main():
    assert nw_score("GATTACA", "GATTACA") == 7  # identical -> len * match
    assert nw_score("GCATGCU", "GATTACA") == 0
    print("needleman-wunsch OK")


if __name__ == "__main__":
    main()
