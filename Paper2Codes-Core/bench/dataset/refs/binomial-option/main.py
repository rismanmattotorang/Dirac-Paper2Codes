"""Cox-Ross-Rubinstein binomial European option pricing (reference)."""

import math


def crr_call(S: float, K: float, r: float, sigma: float, T: float, n: int = 500) -> float:
    dt = T / n
    u = math.exp(sigma * math.sqrt(dt))
    d = 1.0 / u
    p = (math.exp(r * dt) - d) / (u - d)
    disc = math.exp(-r * dt)
    # Terminal option values.
    values = [max(S * (u ** j) * (d ** (n - j)) - K, 0.0) for j in range(n + 1)]
    for _ in range(n):
        values = [disc * (p * values[j + 1] + (1 - p) * values[j]) for j in range(len(values) - 1)]
    return values[0]


def main():
    price = crr_call(S=100, K=100, r=0.05, sigma=0.2, T=1.0, n=500)
    # Converges to the Black-Scholes call (~10.45).
    assert abs(price - 10.45) < 0.1
    print(f"CRR call = {price:.4f}")


if __name__ == "__main__":
    main()
