#!/usr/bin/env python3
"""Package the bundle as a source-only `.hnmod`.

The archive holds `horizon-native.json` and `mixins/<id>/`, which is what
Horizon's registry and launcher read (`horizon_mods::index::native`). Only
sources, manifests and data are included, so nothing is executed to build the
file. Fixed dates, modes, entry order and compression make the bytes (and the
SHA-256) reproducible from the same sources.

Usage: python3 tools/package.py
"""

import hashlib
import json
import re
import zipfile
from pathlib import Path
import tomllib

ROOT = Path(__file__).resolve().parents[1]
GAME = "26.3"
# The text files a bundle may carry beside sources (README, LICENSE, ...).
TEXT_PREFIXES = ("README", "LICENSE", "LICENCE", "COPYING", "NOTICE")


def fail(message: str) -> None:
    raise SystemExit(message)


def allowed(rest: str) -> bool:
    """Whether a path inside `mixins/<id>/` may be in a bundle.

    Mirrors `horizon_mods::index::native::allowed`: Rust and TOML anywhere,
    data under `data/` or one crate folder's `data/`, and the usual text files
    at the bundle root or one folder deep.
    """
    parts = rest.split("/")
    if rest.endswith((".rs", ".toml")):
        return True
    if len(parts) >= 2 and (
        parts[0] == "data" or (len(parts) >= 3 and parts[1] == "data")
    ):
        return True
    name = parts[-1]
    return len(parts) <= 2 and (
        name.startswith(TEXT_PREFIXES) or name == ".gitignore"
    )


def main() -> None:
    compat = json.loads((ROOT / "compatibility.json").read_text())
    manifests = sorted((ROOT / "mixins").glob("*/horizon-mixin.toml"))
    if len(manifests) != 1:
        fail("a .hnmod holds one bundle: keep one mixins/<id>/horizon-mixin.toml")

    bundle = manifests[0].parent
    mixin = tomllib.loads(manifests[0].read_text())
    mod = mixin["mod"]
    if bundle.name != mod["id"]:
        fail(f"rename mixins/{bundle.name}/ to mixins/{mod['id']}/")
    if not re.fullmatch(r"[a-z0-9_]+", mod["id"]):
        fail("the registry takes ids of lowercase letters, digits and _ only")
    if not mod.get("version"):
        fail("horizon-mixin.toml has no [mod].version")

    horizon = mixin.get("compatibility", {}).get("horizon_revision")
    if horizon != compat.get("horizon_revision"):
        fail("horizon-mixin.toml and compatibility.json pin different Horizon revisions")
    mixins = compat.get("horizon_mixins_revision")
    if not re.fullmatch(r"[0-9a-f]{40}", horizon or ""):
        fail("horizon_revision must be a 40-character commit")
    if not re.fullmatch(r"[0-9a-f]{40}", mixins or ""):
        fail("horizon_mixins_revision must be a 40-character commit")

    manifest = {
        "schema": 1,
        "id": mod["id"],
        "version": mod["version"],
        "game": GAME,
        "horizon": horizon,
        "mixins": mixins,
    }
    files = {
        "horizon-native.json": (
            json.dumps(manifest, indent=2, sort_keys=True) + "\n"
        ).encode()
    }
    # The licence and the third-party notice travel with the sources: the
    # bundle reader accepts LICENSE*/NOTICE* text at mixins/<id>/.
    for name, source in (("LICENSE", "LICENSE"), ("NOTICE.md", "THIRD_PARTY_NOTICES.md")):
        path = ROOT / source
        if path.is_file():
            files[f"mixins/{mod['id']}/{name}"] = path.read_bytes()
    for path in sorted(bundle.rglob("*")):
        if path.is_symlink():
            fail(f"symlinks are not allowed in a bundle: {path}")
        if not path.is_file():
            continue
        relative = path.relative_to(bundle).as_posix()
        if not allowed(relative):
            fail(f"not allowed in a bundle: {relative}")
        files[f"mixins/{mod['id']}/{relative}"] = path.read_bytes()

    out = ROOT / "target" / f"{mod['id']}-{mod['version']}.hnmod"
    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name in sorted(files):
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, files[name])

    print(
        json.dumps(
            {
                "file": str(out),
                "size": out.stat().st_size,
                "sha256": hashlib.sha256(out.read_bytes()).hexdigest(),
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
