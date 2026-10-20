# 9. Prediction markets

Catalog mapping: [§9 Prediction markets](../../../../../../../docs/finance-rs/catalog.md#9-prediction-markets).

The `pm` module converts odds, removes overround, calculates venue fees and
contract values, and scores forecasts. Fee schedules are explicit inputs or
named constants; invalid probability domains return errors.
