## Description

<!-- What this changes and why. -->

JIRA: EDGEAI-

## Testing

- [ ] `make lint` and `make test` pass
- [ ] New `#[repr(C)]` structs carry a size assertion
- [ ] Device-backed changes: tested on <!-- vivid / board and driver -->, with the `strace -e trace=ioctl` sequence checked where the ioctl sequence matters

**Devices tested:** <!-- board, driver (VIDIOC_QUERYCAP), kernel -->

**Consumers affected:** <!-- edgefirst-camera, edgefirst-codec; note any API change they must follow -->

## Checklist

- [ ] Branch `feature/EDGEAI-###-desc` or `bugfix/EDGEAI-###-desc`; commits `EDGEAI-###: ...` with `-s`
- [ ] Public items documented (rustdoc)
- [ ] CHANGELOG.md updated for notable changes
- [ ] NOTICE updated when a dependency needing attribution is added (`make sbom`)
