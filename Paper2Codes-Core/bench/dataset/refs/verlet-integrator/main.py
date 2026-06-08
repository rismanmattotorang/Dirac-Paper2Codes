"""Velocity Verlet integrator for the 1D harmonic oscillator (reference)."""


def energy(x: float, v: float, k: float) -> float:
    return 0.5 * v * v + 0.5 * k * x * x


def simulate(x0: float, v0: float, k: float, dt: float, steps: int):
    """Advance (x, v) by `steps` velocity-Verlet steps; return (x, v)."""
    x, v = x0, v0
    a = -k * x
    for _ in range(steps):
        x = x + v * dt + 0.5 * a * dt * dt
        a_next = -k * x
        v = v + 0.5 * (a + a_next) * dt
        a = a_next
    return x, v


def main():
    k, dt = 1.0, 0.01
    x0, v0 = 1.0, 0.0
    e0 = energy(x0, v0, k)
    x, v = simulate(x0, v0, k, dt, steps=10_000)
    # Symplectic integrator conserves energy to high accuracy.
    assert abs(energy(x, v, k) - e0) < 1e-2
    print(f"final x={x:.4f} v={v:.4f}, energy drift={abs(energy(x, v, k) - e0):.2e}")


if __name__ == "__main__":
    main()
