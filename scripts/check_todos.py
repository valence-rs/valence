import pathlib
import re


def check(lines: list[str]) -> list[tuple[int, str]]:
    results = []
    for (i, line) in enumerate(lines):
        result = re.search(r"((//|/\*|///).*TODO|todo!)(?!.*\(#\d+\))", line)
        if result:
            results.append((i, line))
    return results

if __name__ == "__main__":
    paths = list(pathlib.Path(".").rglob("*"))

    clean = True
    for path in paths:
        if path.is_file():
            with open(path, "r") as f:
                try:
                    lines = f.readlines()
                    for (i, line) in check(lines):
                        if clean:
                            clean = False
                        print(f"[{path}:{i}] {line}", end="")
                except UnicodeDecodeError:
                    continue
    
    if not clean:
        print("\nYour code has TODOs that don't reference any issues! Create issues for your todos and reference them like this: `(#<issue number>)`. Example: // TODO: Foo (#123)")
        exit(1)