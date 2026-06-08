"""1D linear advection via first-order upwind scheme (reference)."""

from typing import List


def step(u: List[float], c: float, dx: float, dt: float) -> List[float]:
    cfl = c * dt / dx
    if cfl > 1.0:
        raise ValueError(f"CFL condition violated: {cfl} > 1")
    n = len(u)
    return [u[i] - cfl * (u[i] - u[(i - 1) % n]) for i in range(n)]  # periodic


def solve(u0: List[float], c: float, dx: float, dt: float, steps: int) -> List[float]:
    u = list(u0)
    for _ in range(steps):
        u = step(u, c, dx, dt)
    return u


def main():
    const = [1.0] * 10
    out = solve(const, c=1.0, dx=1.0, dt=0.5, steps=20)
    # A constant profile is preserved exactly by advection.
    assert all(abs(v - 1.0) < 1e-12 for v in out)
    print("advection-1d OK")


if __name__ == "__main__":
    main()
