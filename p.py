import re


def strip_garbage(data: bytes) -> str:
    """Decode UTF-16LE bytes and strip control chars and progress bar block elements."""
    text = data.decode('utf-16-le', errors='replace')
    # Remove ANSI escape sequences
    text = re.sub(r'\x1b\[[0-9;]*[mK]', '', text)
    # Remove C0/C1 control chars (but keep \n and space)
    text = re.sub(r'[\x00-\x08\x0B-\x1F\x7F-\x9F]', '', text)
    # Remove Unicode box-drawing and block elements used in progress bars
    text = re.sub(r'[\u2500-\u25FF]', '', text)
    return text


def parse_winget_table(filepath):
    with open(filepath, 'rb') as f:
        data = f.read()

    text = strip_garbage(data)
    lines = text.splitlines()

    # Find the table separator line (long continuous run of dashes)
    sep_idx = next(
        (i for i, line in enumerate(lines) if re.match(r'\s*-{10,}\s*$', line)),
        None
    )
    if sep_idx is None:
        print("No table found")
        return []

    header_line = lines[sep_idx - 1]

    # Derive column start positions from the header: gaps of 2+ spaces mark column boundaries
    col_starts = [0]
    for m in re.finditer(r' {2,}', header_line):
        col_starts.append(m.end())
    col_starts.append(len(header_line) + 1)

    col_ranges = list(zip(col_starts, col_starts[1:]))

    def extract_row(line):
        return [line[s:e].strip() if s < len(line) else '' for s, e in col_ranges]

    headers = [item.lower() for item in extract_row(header_line)]

    rows = []
    for line in lines[sep_idx + 1:]:
        stripped = line.strip()
        if not stripped:
            continue
        # Summary line like "2 upgrades available." — starts with a digit
        if re.match(r'^\d+ ', stripped):
            break
        cols = extract_row(line)
        rows.append(dict(zip(headers, cols)))

    return rows


if __name__ == '__main__':
    rows = parse_winget_table('winget-stub/w-upgrade.txt')
    for row in rows:
        print(row)
