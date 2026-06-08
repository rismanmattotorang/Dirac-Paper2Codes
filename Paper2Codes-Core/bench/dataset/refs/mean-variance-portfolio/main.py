"""Global minimum-variance portfolio (reference implementation)."""

import numpy as np


def gmv_weights(cov: np.ndarray) -> np.ndarray:
    """Return the global minimum-variance weights for covariance matrix `cov`."""
    cov = np.asarray(cov, dtype=float)
    n = cov.shape[0]
    ones = np.ones(n)
    inv = np.linalg.inv(cov)
    w = inv @ ones
    return w / (ones @ inv @ ones)


def portfolio_variance(cov: np.ndarray, w: np.ndarray) -> float:
    return float(w @ np.asarray(cov, dtype=float) @ w)


def main():
    cov = np.array([[0.04, 0.01, 0.0], [0.01, 0.09, 0.0], [0.0, 0.0, 0.16]])
    w = gmv_weights(cov)
    assert abs(w.sum() - 1.0) < 1e-9  # fully invested

    equal = np.ones(3) / 3
    assert portfolio_variance(cov, w) <= portfolio_variance(cov, equal) + 1e-12
    print("GMV weights:", np.round(w, 4))


if __name__ == "__main__":
    main()
