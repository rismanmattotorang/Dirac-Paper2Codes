"""Inventory reorder point with safety stock (reference)."""

import math


def reorder_point(daily_demand: float, lead_time_days: float,
                  demand_std: float = 0.0, z: float = 1.65):
    safety_stock = z * demand_std * math.sqrt(lead_time_days)
    rop = daily_demand * lead_time_days + safety_stock
    return {"reorder_point": rop, "safety_stock": safety_stock}


def main():
    # Zero variability -> ROP = demand * lead time.
    r = reorder_point(daily_demand=50, lead_time_days=4, demand_std=0.0)
    assert abs(r["reorder_point"] - 200.0) < 1e-9
    assert r["safety_stock"] == 0.0
    print(f"ROP = {r['reorder_point']:.1f}")


if __name__ == "__main__":
    main()
