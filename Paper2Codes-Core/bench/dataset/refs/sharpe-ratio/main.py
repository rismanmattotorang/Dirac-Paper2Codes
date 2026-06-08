"""Sharpe ratio of a return series (reference)."""

import math


def sharpe_ratio(returns, risk_free=0.0, periods_per_year=1):
    excess = [r - risk_free for r in returns]
    n = len(excess)
    if n == 0:
        return 0.0
    mean = sum(excess) / n
    var = sum((x - mean) ** 2 for x in excess) / n
    std = math.sqrt(var)
    if std == 0.0:
        return 0.0
    return (mean / std) * math.sqrt(periods_per_year)


def main():
    rets = [0.01, 0.02, -0.01, 0.03, 0.00]
    s = sharpe_ratio(rets, risk_free=0.0, periods_per_year=252)
    assert sharpe_ratio([0.01, 0.01, 0.01]) == 0.0  # zero variance
    print(f"annualised Sharpe = {s:.3f}")


if __name__ == "__main__":
    main()
