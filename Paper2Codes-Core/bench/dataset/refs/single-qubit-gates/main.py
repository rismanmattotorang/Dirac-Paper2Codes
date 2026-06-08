"""Single-qubit X and Hadamard gates on |0> (reference)."""

import numpy as np

X = np.array([[0, 1], [1, 0]], dtype=complex)
H = (1 / np.sqrt(2)) * np.array([[1, 1], [1, -1]], dtype=complex)


def apply_gates(gates):
    state = np.array([1, 0], dtype=complex)  # |0>
    for g in gates:
        state = g @ state
    return state


def main():
    assert np.allclose(apply_gates([X]), np.array([0, 1]))  # X|0> = |1>
    h0 = apply_gates([H])
    assert abs(np.sum(np.abs(h0) ** 2) - 1.0) < 1e-9  # normalised
    print("single-qubit gates OK")


if __name__ == "__main__":
    main()
