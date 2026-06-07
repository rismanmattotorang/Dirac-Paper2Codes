"""Black-Scholes European option pricing (reference implementation)."""

import math


def _norm_cdf(x: float) -> float:
    """Standard normal CDF via the error function."""
    return 0.5 * (1.0 + math.erf(x / math.sqrt(2.0)))


def black_scholes(S: float, K: float, r: float, sigma: float, T: float):
    """Return (call, put) prices for a European option.

    S: spot, K: strike, r: risk-free rate, sigma: volatility, T: time to maturity.
    """
    if T <= 0 or sigma <= 0:
        raise ValueError("T and sigma must be positive")
    d1 = (math.log(S / K) + (r + 0.5 * sigma * sigma) * T) / (sigma * math.sqrt(T))
    d2 = d1 - sigma * math.sqrt(T)
    call = S * _norm_cdf(d1) - K * math.exp(-r * T) * _norm_cdf(d2)
    put = K * math.exp(-r * T) * _norm_cdf(-d2) - S * _norm_cdf(-d1)
    return call, put


def main():
    call, put = black_scholes(S=100.0, K=100.0, r=0.05, sigma=0.2, T=1.0)
    # Put-call parity: C - P == S - K * exp(-r T)
    parity = call - put
    expected = 100.0 - 100.0 * math.exp(-0.05 * 1.0)
    assert abs(parity - expected) < 1e-9
    print(f"call={call:.4f} put={put:.4f}")


if __name__ == "__main__":
    main()
