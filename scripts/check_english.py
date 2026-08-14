from pathlib import Path

root = Path(__file__).resolve().parents[1]
ignored = {".git", "target", "node_modules", "dist", "__pycache__"}
violations: list[tuple[str, int]] = []
for path in root.rglob("*"):
    if not path.is_file() or any(part in ignored for part in path.parts):
        continue
    try:
        text = path.read_text(encoding="utf-8")
    except (UnicodeDecodeError, OSError):
        continue
    for line_number, line in enumerate(text.splitlines(), 1):
        if any("\u0600" <= char <= "\u06ff" for char in line):
            violations.append((str(path.relative_to(root)), line_number))

if violations:
    for path, line in violations:
        print(f"{path}:{line}")
    raise SystemExit("Arabic text found in project files")

print("English-only project check passed")
