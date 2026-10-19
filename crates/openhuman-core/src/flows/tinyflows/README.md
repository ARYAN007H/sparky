# TinyFlows graph vocabulary

`mod.rs` keeps a small compatibility namespace for the generic graph model and
legacy graph migration used by OpenHuman's authoring, validation, and Catalog
adapters.

Execution is owned by the compiled TinyFlows module and accessed through its
TinyBus contracts. OpenHuman must not call TinyFlows compiler, engine,
capability, checkpoint, run-row, or observer implementations directly. The
host-side code in `flows/` is limited to contract adapters and OpenHuman policy
callbacks.
