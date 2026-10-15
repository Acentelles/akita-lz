# Fixed protocol regression schedules

`fp32_onehot_tensor_nv16_np2.aks` pins the scalar fp32 one-hot, nv16,
two-polynomial row from `fddf785cf1d6011b508c271d78e69958d1fde4af`,
rebound to the current sole policy identity. It has zero setup offloads and
contains a tensor successor. `portable_tensor_import_matches_homogeneous_proof`
loads it through normal scheme admission, then checks batched transfer between
CPU executors against homogeneous proof bytes and verification.

To reconstruct it, extract that single row from the revision's
`artifacts/schedules/fp32_onehot.aks`, retain its envelope and schedule, and serialize it with `ValidatedScheduleCatalog::try_new` and
`to_artifact_bytes` under the current fp32 one-hot policy. After a policy identity
change, update only that envelope field, preserving canonical row formatting (see
`crates/akita-config/tests/fixtures/README.md`), and run:

```sh
rtk cargo nextest run -p akita-pcs --test dynamic_backends \
  -E 'test(portable_tensor_import_matches_homogeneous_proof)'
```

The test must still pass independent admission, exercise two polynomials, select
zero offloads, and transfer a tensor successor. If policy constraints change,
rebuild an admitted row preserving those properties; changing the digest cannot
make an invalid schedule valid. Optimizer changes alone do not require updating
this fixture.
