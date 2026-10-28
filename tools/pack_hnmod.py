"""Pack the registry's .hnmod: horizon-native.json + mixins/<id>/*.{rs,toml}.

Deterministic, mirroring the layout of published bundles: no directory
entries, sorted names, deflate, fixed 2026-01-01 timestamps, Unix 0644.
Usage: python tools/pack_hnmod.py [OUT]  (default: dist/<id>-<version>.hnmod)
"""
import json
import pathlib
import sys
import zipfile

root = pathlib.Path(__file__).resolve().parent.parent
manifest = json.loads((root / "horizon-native.json").read_text(encoding="utf-8"))
mod = root / "mixins" / manifest["id"]
names = ["horizon-native.json"] + [
    f"mixins/{manifest['id']}/{p.name}"
    for p in mod.iterdir()
    if p.is_file() and p.suffix in (".rs", ".toml")
]
out = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else (
    root / "dist" / f"{manifest['id']}-{manifest['version']}.hnmod"
)
out.parent.mkdir(parents=True, exist_ok=True)
with zipfile.ZipFile(out, "w") as z:
    for name in sorted(names):
        info = zipfile.ZipInfo(name, date_time=(2026, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        info.create_system = 3
        info.external_attr = 0o100644 << 16
        z.writestr(info, (root / name).read_bytes())
print(out)
