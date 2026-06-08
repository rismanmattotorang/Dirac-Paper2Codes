"""Plane Poiseuille flow velocity profile (reference)."""

from typing import List


def velocity_profile(h: float, mu: float, dpdx: float, points: int = 11) -> List[float]:
    ys = [-h + 2 * h * i / (points - 1) for i in range(points)]
    return [-(1.0 / (2 * mu)) * dpdx * (h * h - y * y) for y in ys]


def main():
    u = velocity_profile(h=1.0, mu=1.0, dpdx=-2.0, points=11)
    # No-slip at the walls (first and last points), maximum at the centre.
    assert abs(u[0]) < 1e-12 and abs(u[-1]) < 1e-12
    assert u[len(u) // 2] == max(u)
    print("poiseuille-flow OK")


if __name__ == "__main__":
    main()
