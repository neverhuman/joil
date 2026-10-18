#!/usr/bin/env python3
"""Exercise D-29 frozen source bindings and blind-judge dependency boundary."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
FROZEN = "e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0"
SOURCES = {
    "crates/joil-core/src/evolve.rs": "62cb5c8c30186155d09fb816a0e42ad7da615738cfdc11cb27348a16f94778ab",
    "crates/joil-ir/src/lib.rs": "8926748a21966751dc40d12a7ab4227b294cb7dd05b6ec9a9b7949aca1be43ec",
}
MANIFESTS = (
    "packs/hydrocyclone/judges/Cargo.toml",
    "crates/crucible-fea/Cargo.toml",
    "crates/crucible-lbm/Cargo.toml",
    "crates/crucible-particles/Cargo.toml",
    "crates/crucible-architecture/Cargo.toml",
)
FORBIDDEN = {"crucible-genome", "crucible-search"}
JUDGE = "pack-hydrocyclone-judges"


def check_frozen_sources():
    subprocess.run(["git", "cat-file", "-e", FROZEN + "^{commit}"], cwd=ROOT,
                   check=True, stdout=subprocess.DEVNULL)
    for path, expected in SOURCES.items():
        data = subprocess.run(["git", "show", f"{FROZEN}:{path}"], cwd=ROOT,
                              check=True, stdout=subprocess.PIPE).stdout
        actual = hashlib.sha256(data).hexdigest()
        if actual != expected:
            raise ValueError(f"frozen source digest mismatch for {path}: {actual}")


def cargo_graph():
    for manifest in MANIFESTS:
        if not (ROOT / manifest).is_file():
            raise ValueError(f"missing workspace member manifest: {manifest}")
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--offline", "--all-features", "--format-version", "1"],
        cwd=ROOT, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    metadata = json.loads(result.stdout)
    packages = {package["id"]: package["name"] for package in metadata["packages"]}
    graph = {name: set() for name in packages.values()}
    for node in metadata["resolve"]["nodes"]:
        source = packages[node["id"]]
        for dependency in node["deps"]:
            target = packages[dependency["pkg"]]
            graph[source].add(target)
    if JUDGE not in graph:
        raise ValueError(f"blind-judge crate missing from resolved workspace: {JUDGE}")
    return graph


def forbidden_path(graph, root):
    pending = [(root, [root])]
    visited = set()
    while pending:
        package, path = pending.pop()
        if package in FORBIDDEN:
            return path
        if package in visited:
            continue
        visited.add(package)
        pending.extend((child, path + [child]) for child in sorted(graph.get(package, ())))
    return None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=("positive", "red"), required=True)
    mode = parser.parse_args().mode
    try:
        check_frozen_sources()
        graph = cargo_graph()
        if mode == "positive":
            bad = forbidden_path(graph, JUDGE)
            if bad:
                raise ValueError("blind judge reaches forbidden package: " + " -> ".join(bad))
            print(f"D-29 positive PASS: {JUDGE} has no transitive path to {', '.join(sorted(FORBIDDEN))}")
            return 0

        # Synthetic two-edge path exercises the traversal without changing manifests.
        red_graph = {name: set(children) for name, children in graph.items()}
        red_graph[JUDGE].add("synthetic-d29-bridge")
        red_graph["synthetic-d29-bridge"] = {"crucible-genome"}
        bad = forbidden_path(red_graph, JUDGE)
        if not bad:
            raise ValueError("red control failed to reject synthetic indirect dependency")
        print("D-29 RED reproduced: " + " -> ".join(bad), file=sys.stderr)
        return 1
    except (OSError, ValueError, subprocess.CalledProcessError, KeyError, json.JSONDecodeError) as error:
        print(f"D-29 control ERROR: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
