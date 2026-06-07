"""Two-qubit Bell state preparation (reference implementation).

Pure-numpy state-vector simulation so the reference runs without a quantum SDK;
an equivalent Qiskit/Cirq circuit (H on q0, then CNOT 0->1) is the idiomatic form.
"""

import numpy as np


def bell_state() -> np.ndarray:
    """Prepare |Phi+> = (|00> + |11>) / sqrt(2) and return the state vector."""
    # Basis ordering: |00>, |01>, |10>, |11>
    zero_zero = np.array([1, 0, 0, 0], dtype=complex)

    h = (1 / np.sqrt(2)) * np.array([[1, 1], [1, -1]], dtype=complex)
    identity = np.eye(2, dtype=complex)
    cnot = np.array(
        [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 0, 1], [0, 0, 1, 0]], dtype=complex
    )

    state = np.kron(h, identity) @ zero_zero  # H on qubit 0
    state = cnot @ state  # CNOT control=0 target=1
    return state


def main():
    state = bell_state()
    probs = np.abs(state) ** 2
    assert abs(probs.sum() - 1.0) < 1e-9  # normalised
    # Only |00> and |11> have amplitude.
    assert abs(probs[0] - 0.5) < 1e-9 and abs(probs[3] - 0.5) < 1e-9
    assert probs[1] < 1e-9 and probs[2] < 1e-9
    print("Bell state |Phi+> prepared:", np.round(state, 3))


if __name__ == "__main__":
    main()
