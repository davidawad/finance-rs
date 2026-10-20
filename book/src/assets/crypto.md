# 8. Crypto

Catalog mapping: [§8 Crypto](../../../../../../../docs/finance-rs/catalog.md#8-crypto).

Crypto spot, funding, and token conventions are caller-specific. Use the
generic cash-flow and rate APIs to model dated funding or proceeds, and keep
exchange-specific fees outside the generic math unless they match a supported
prediction-market fee helper.
