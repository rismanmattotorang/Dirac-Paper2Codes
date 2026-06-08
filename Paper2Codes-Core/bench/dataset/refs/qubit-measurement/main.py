"""Computational-basis measurement probabilities (Born rule, reference)."""

import numpy as np


def measurement_probabilities(amplitudes):
    psi = np.asarray(amplitudes, dtype=complex)
    norm = float(np.sum(np.abs(psi) ** 2))
    if abs(norm - 1.0) > 1e-6:
        raise ValueError("state is not normalised")
    return np.abs(psi) ** 2


def main():
    p = measurement_probabilities([1 / np.sqrt(2), 1 / np.sqrt(2)])
    assert abs(p.sum() - 1.0) < 1e-9
    assert np.all(p >= 0)
    print("probabilities:", np.round(p, 3))


if __name__ == "__main__":
    main()
