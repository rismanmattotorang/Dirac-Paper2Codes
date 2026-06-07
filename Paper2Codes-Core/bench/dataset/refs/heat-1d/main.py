"""1D heat equation via explicit FTCS finite differences (reference)."""

from typing import List


def step(u: List[float], alpha: float, dx: float, dt: float) -> List[float]:
    """Advance one FTCS step with fixed (Dirichlet) boundaries."""
    cfl = alpha * dt / (dx * dx)
    if cfl > 0.5:
        raise ValueError(f"CFL condition violated: {cfl} > 0.5")
    n = len(u)
    nxt = list(u)
    for i in range(1, n - 1):
        nxt[i] = u[i] + cfl * (u[i + 1] - 2.0 * u[i] + u[i - 1])
    return nxt


def solve(u0: List[float], alpha: float, dx: float, dt: float, steps: int) -> List[float]:
    """Advance the initial profile u0 by `steps` time steps."""
    u = list(u0)
    for _ in range(steps):
        u = step(u, alpha, dx, dt)
    return u


def main():
    u0 = [0.0, 1.0, 1.0, 1.0, 0.0]
    total0 = sum(u0)
    u = solve(u0, alpha=1.0, dx=1.0, dt=0.25, steps=10)
    # Interior heat dissipates to the fixed boundaries; total stays bounded.
    assert sum(u) <= total0 + 1e-9
    print("final profile:", [round(x, 4) for x in u])


if __name__ == "__main__":
    main()
