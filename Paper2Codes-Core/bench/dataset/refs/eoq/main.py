"""Economic Order Quantity (EOQ) model (reference implementation)."""

import math


def eoq(demand: float, order_cost: float, holding_cost: float):
    """Return (Q_star, total_cost) for the classic EOQ model."""
    if demand <= 0 or order_cost <= 0 or holding_cost <= 0:
        raise ValueError("demand, order_cost, holding_cost must be positive")
    q_star = math.sqrt(2.0 * demand * order_cost / holding_cost)
    total_cost = (demand / q_star) * order_cost + (q_star / 2.0) * holding_cost
    return q_star, total_cost


def main():
    q, tc = eoq(demand=1000.0, order_cost=50.0, holding_cost=2.0)
    # At the optimum, ordering cost == holding cost and TC == sqrt(2 D S H).
    assert abs(tc - math.sqrt(2.0 * 1000.0 * 50.0 * 2.0)) < 1e-6
    print(f"Q* = {q:.2f}, total cost = {tc:.2f}")


if __name__ == "__main__":
    main()
