"""Lennard-Jones (12-6) potential (reference implementation)."""


def lennard_jones(r: float, epsilon: float = 1.0, sigma: float = 1.0) -> float:
    """Return the Lennard-Jones interaction energy V(r)."""
    if r <= 0:
        raise ValueError("separation r must be positive")
    sr6 = (sigma / r) ** 6
    return 4.0 * epsilon * (sr6 * sr6 - sr6)


def well_minimum(epsilon: float = 1.0, sigma: float = 1.0):
    """Return (r_min, V_min) of the potential well."""
    r_min = 2.0 ** (1.0 / 6.0) * sigma
    return r_min, -epsilon


def main():
    r_min, v_min = well_minimum(epsilon=1.0, sigma=1.0)
    assert abs(lennard_jones(r_min) - v_min) < 1e-9
    print(f"V({r_min:.4f}) = {lennard_jones(r_min):.4f} (well depth = {v_min})")


if __name__ == "__main__":
    main()
