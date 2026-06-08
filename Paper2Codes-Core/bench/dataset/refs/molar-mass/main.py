"""Molar mass from a chemical formula (reference)."""

import re

ATOMIC_WEIGHTS = {
    "H": 1.008, "C": 12.011, "N": 14.007, "O": 15.999,
    "Na": 22.990, "Cl": 35.45, "S": 32.06, "P": 30.974,
}


def molar_mass(formula: str) -> float:
    total = 0.0
    for element, count in re.findall(r"([A-Z][a-z]?)(\d*)", formula):
        if not element:
            continue
        if element not in ATOMIC_WEIGHTS:
            raise ValueError(f"unknown element: {element}")
        total += ATOMIC_WEIGHTS[element] * (int(count) if count else 1)
    return total


def main():
    assert abs(molar_mass("H2O") - 18.015) < 1e-3
    assert abs(molar_mass("NaCl") - 58.44) < 1e-2
    print(f"M(H2O) = {molar_mass('H2O'):.3f} g/mol")


if __name__ == "__main__":
    main()
