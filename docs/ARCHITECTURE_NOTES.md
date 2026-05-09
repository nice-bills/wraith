# Architecture Notes

This project should not treat Self as the only reference.

## Reference Set

- `Self` - best end-to-end product flow and verification UX.
- `Rarimo` - strong passport circuit and document parsing reference.
- `OpenPassport` - predecessor architecture and mobile capture flow.
- `ZKPassport` - alternate implementation style for passport proofs.
- `Mopro` - mobile proving path to study before committing to TEE-only proving.

## What To Borrow

- Keep the Stellar contract as the system of record.
- Keep the proof adapter as the boundary between Groth16 artifacts and Soroban payloads.
- Keep the SDK thin and opinionated.
- Keep the app policy model reusable across apps.
- Treat mobile proving as a future architecture option, not a blocker for launch.

## Current Build Direction

The current repo should stay focused on a Stellar-native identity layer with two proof modes:

1. On-chain Groth16 verification for direct contract validation.
2. Attested prover submission for TEE-backed or service-backed proof flows.

That gives us a launch path now, while preserving room to move toward client-side/mobile proving later.

## Practical Constraint

The first shippable version must be boring:

- clear policy registry
- replay protection
- reproducible payload conversion
- deployable contract
- minimal SDK surface

The interesting cryptography stays, but the repo should look like infrastructure, not a research dump.
