# Treasury, Digital Family, and Inheritance Hooks (Phase 20 Track)

## Intent
Support constitutional concepts (treasury, digital family, inheritance) without central capture.

## Key primitives
- **Treasury Accounts**: special subjects controlled by quorum governance.
- **Family Vault**: a subject with multiple guardians.
- **Inheritance Plan**: time-locked or condition-locked rules that can transfer control.

## Rules (MVP)
- Plans are **proposal-only** until Phase 21+.
- Any transfer of treasury/family assets requires:
  - quorum approval
  - audit record
  - dispute window

## Privacy
- Public logs store only:
  - digests
  - policy refs
  - minimal metadata
- Private details are encrypted and revealed only to juries/authorized flows.
