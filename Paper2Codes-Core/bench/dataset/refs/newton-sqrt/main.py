"""Newton's method for square roots (reference implementation)."""


def newton_sqrt(a: float, tol: float = 1e-12, max_iter: int = 100) -> float:
    """Return sqrt(a) via Newton's iteration x_{n+1} = 0.5 * (x_n + a / x_n)."""
    if a < 0:
        raise ValueError("a must be non-negative")
    if a == 0:
        return 0.0
    x = a if a >= 1.0 else 1.0  # positive initial guess
    for _ in range(max_iter):
        nxt = 0.5 * (x + a / x)
        if abs(nxt - x) < tol:
            return nxt
        x = nxt
    return x


def main():
    for a in (0.0, 2.0, 9.0, 1e6):
        r = newton_sqrt(a)
        assert abs(r * r - a) < 1e-6
        print(f"sqrt({a}) = {r}")


if __name__ == "__main__":
    main()
