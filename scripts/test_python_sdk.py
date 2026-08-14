import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "sdk" / "python"))
import causa

tape = causa.start("sdk-test")
tape.event("note", "hello", {"x": 1}, {"status": "ok"})
out = Path("/tmp/sdk-test.causa")
tape.write(out)
assert out.exists()
print(out)
