"""pH from hydrogen-ion concentration (reference)."""

import math


def ph(h_concentration: float):
    if h_concentration <= 0:
        raise ValueError("[H+] must be positive")
    p = -math.log10(h_concentration)
    return {"pH": p, "pOH": 14.0 - p}


def main():
    r = ph(1e-7)
    assert abs(r["pH"] - 7.0) < 1e-9  # neutral
    assert abs(ph(1e-3)["pH"] - 3.0) < 1e-9
    print(f"pH(1e-7) = {r['pH']:.2f}")


if __name__ == "__main__":
    main()
