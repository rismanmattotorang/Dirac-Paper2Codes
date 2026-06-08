"""Projectile motion under uniform gravity (reference)."""

import math


def projectile(v0: float, theta_deg: float, g: float = 9.81):
    theta = math.radians(theta_deg)
    tof = 2 * v0 * math.sin(theta) / g
    rng = v0 * v0 * math.sin(2 * theta) / g
    h = (v0 * math.sin(theta)) ** 2 / (2 * g)
    return {"time_of_flight": tof, "range": rng, "max_height": h}


def main():
    # Maximum range is at 45 degrees.
    r45 = projectile(20, 45)["range"]
    assert r45 >= projectile(20, 30)["range"]
    assert r45 >= projectile(20, 60)["range"]
    print(f"range@45 = {r45:.3f} m")


if __name__ == "__main__":
    main()
