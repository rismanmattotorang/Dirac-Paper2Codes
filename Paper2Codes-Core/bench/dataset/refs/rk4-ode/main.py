"""Fourth-order Runge-Kutta ODE integration (reference)."""

import math


def rk4(f, t0, y0, t1, steps):
    h = (t1 - t0) / steps
    t, y = t0, y0
    for _ in range(steps):
        k1 = f(t, y)
        k2 = f(t + h / 2, y + h * k1 / 2)
        k3 = f(t + h / 2, y + h * k2 / 2)
        k4 = f(t + h, y + h * k3)
        y += (h / 6) * (k1 + 2 * k2 + 2 * k3 + k4)
        t += h
    return y


def main():
    # dy/dt = y, y(0)=1 -> y(1)=e
    y = rk4(lambda t, y: y, 0.0, 1.0, 1.0, 100)
    assert abs(y - math.e) < 1e-6
    print(f"y(1) = {y:.8f} (e = {math.e:.8f})")


if __name__ == "__main__":
    main()
