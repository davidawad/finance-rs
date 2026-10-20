# 1. Core

Catalog mapping: [§1 Core](../../../../../../../docs/finance-rs/catalog.md#1-core).

Core dates, day counts, schedules, cash flows, summary statistics, and root
solvers support the higher-level asset modules. Keep validation at the edge:
use `Date::new` for checked dates and solver option types when you need
non-default tolerances.
