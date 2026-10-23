from pathlib import Path
import hashlib
root = Path(__file__).resolve().parent
count = 0
for line in (root / "MANIFEST.sha256").read_text(encoding="utf-8").splitlines():
    expected, relative = line.split("  ", 1)
    path = (root / relative).resolve()
    assert path.is_relative_to(root), relative
    assert hashlib.sha256(path.read_bytes()).hexdigest() == expected, relative
    count += 1
print(f"Verified {count} distributed files")
