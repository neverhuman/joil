#!/usr/bin/env python3
"""Source-bound software control for D-28 coordination ownership policy."""
import argparse
import hashlib
import subprocess
import sys

BASELINE = "e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0"
SOURCES = {
    "docs/COORDINATION.md": "08b88ff8ea3853745fac524fa387f6dc90ed65b441e49b24f228b2d40ca72b0a",
    "COMMS.md": "f64dd4d8a716ac65b2c10c21f79147eda749560dc8d9ec73fbf7674d24a6af0b",
}


def verify_baseline():
    subprocess.run(["git", "cat-file", "-e", f"{BASELINE}^{{commit}}"], check=True)
    print(f"verified frozen source commit {BASELINE}")
    for path, expected in SOURCES.items():
        content = subprocess.run(["git", "show", f"{BASELINE}:{path}"],
                                 check=True, stdout=subprocess.PIPE).stdout
        actual = hashlib.sha256(content).hexdigest()
        if actual != expected:
            raise ValueError(f"frozen source digest mismatch: {path}: {actual}")
        print(f"verified {path} sha256={actual}")


def allocation_passes(allocation):
    paths = allocation["write_scope"]
    other_scopes = allocation["other_active_write_scopes"]
    disjoint = all(not (a == b or a.startswith(b.rstrip("/") + "/")
                        or b.startswith(a.rstrip("/") + "/"))
                   for a in paths for scope in other_scopes for b in scope)
    return (allocation.get("target_worker_acknowledged") is True
            and allocation.get("lease_active") is True
            and bool(paths) and disjoint)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=("positive", "red"), required=True)
    mode = parser.parse_args().mode
    try:
        verify_baseline()
        common = {
            "write_scope": ["example/worker-owned.py"],
            "other_active_write_scopes": [["example/integrator-owned.py"]],
            "lease_active": True,
        }
        if mode == "positive":
            case = {**common, "target_worker_acknowledged": True}
            if not allocation_passes(case):
                raise ValueError("acknowledged, leased, disjoint allocation rejected")
            print("PASS: synthetic allocation with target acknowledgement, active lease, "
                  "and disjoint write scope accepted")
            return 0
        case = {**common, "target_worker_acknowledged": False}
        if allocation_passes(case):
            raise ValueError("unacknowledged proposed allocation was incorrectly accepted")
        print("PASS: synthetic allocation proposal without target acknowledgement rejected")
        # Red mode represents the rejected allocation, therefore the control exits 1.
        return 1
    except (OSError, subprocess.CalledProcessError, ValueError, KeyError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
