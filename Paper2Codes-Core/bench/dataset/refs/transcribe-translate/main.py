"""DNA transcription and translation (reference)."""

CODON_TABLE = {
    "AUG": "M", "UUU": "F", "UUC": "F", "GGU": "G", "GGC": "G", "GGA": "G",
    "UAA": "*", "UAG": "*", "UGA": "*", "AAA": "K", "GAU": "D", "GAA": "E",
}


def transcribe(dna: str) -> str:
    return dna.upper().replace("T", "U")


def translate(rna: str) -> str:
    protein = []
    for i in range(0, len(rna) - 2, 3):
        codon = rna[i : i + 3]
        aa = CODON_TABLE.get(codon, "X")
        if aa == "*":
            break
        protein.append(aa)
    return "".join(protein)


def main():
    rna = transcribe("ATGGGAAAATAA")
    assert rna == "AUGGGAAAAUAA"
    protein = translate(rna)
    assert protein.startswith("M") and "*" not in protein
    print(f"protein = {protein}")


if __name__ == "__main__":
    main()
