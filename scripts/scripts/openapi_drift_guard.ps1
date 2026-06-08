param(
  [string]$Root = "."
)
python scripts/guards/openapi_drift_guard.py $Root --service all
