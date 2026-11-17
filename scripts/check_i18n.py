import re
import sys
from pathlib import Path

src = Path(__file__).resolve().parent.parent / "web" / "src"
entry = re.compile(r"^  ([a-z0-9_]+): '((?:[^'\\]|\\.)*)',$", re.M)


def load(name):
    return dict(entry.findall((src / "lib" / "lang" / f"{name}.ts").read_text()))


langs = {name: load(name) for name in ("de", "en", "da")}
problems = []
base = set(langs["de"])
for name, texts in langs.items():
    for key in sorted(base ^ set(texts)):
        problems.append(f"{name}: key set differs at {key}")
    for key, text in texts.items():
        if key in langs["de"] and set(re.findall(r"\{\w+\}", text)) != set(re.findall(r"\{\w+\}", langs["de"][key])):
            problems.append(f"{name}: placeholders differ in {key}")

code = ""
for path in list(src.rglob("*.svelte")) + list(src.rglob("*.ts")):
    if "lang" not in path.parts:
        code += path.read_text()
for key in sorted(set(re.findall(r"\bt\(\s*'([^']*)'", code)) - base):
    problems.append(f"unknown key in t(): {key}")
for key in sorted(base):
    if not key.startswith(("error_", "badge_")) and f"'{key}'" not in code:
        problems.append(f"unused key: {key}")

for name in langs:
    path = src / "lib" / "lang" / f"{name}.ts"
    lines = path.read_text().split("\n")
    body = [line for line in lines if entry.match(line)]
    if body != sorted(body, key=lambda line: line.split(":")[0]):
        problems.append(f"{name}: keys are not sorted")

print("\n".join(problems) or f"ok, {len(base)} keys")
sys.exit(1 if problems else 0)
