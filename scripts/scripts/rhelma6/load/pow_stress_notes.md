# PoW Stress Notes

For realistic stress testing, use `rhelma-node register` with PoW enabled and vary:
- difficulty bits
- challenge TTL
- rate-limit max

The recommended approach is to increase difficulty until median solve time is ~200–600ms on typical machines.
