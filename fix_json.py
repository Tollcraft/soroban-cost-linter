import json

with open("docs/lints/lint-registry.json", "r") as f:
    lines = f.readlines()

new_lines = []
skip = False
for line in lines:
    if line.startswith("<<<<<<<"):
        new_lines.append('    "description": "stores an Option<T> in storage where the key already models absence",\n')
        new_lines.append('    "docs_path": "docs/lints/option_wrapping_in_storage.md",\n')
        new_lines.append('    "name": "option_wrapping_in_storage"\n')
        new_lines.append('  },\n')
        new_lines.append('  {\n')
        new_lines.append('    "category": "Other",\n')
        new_lines.append('    "default_level": "warn",\n')
        skip = True
    elif line.startswith("======="):
        skip = False
    elif line.startswith(">>>>>>>"):
        pass
    else:
        if not skip:
            new_lines.append(line)

with open("docs/lints/lint-registry.json", "w") as f:
    f.writelines(new_lines)
