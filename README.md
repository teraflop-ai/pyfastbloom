# pyfastbloom

Python bindings for [fastbloom](https://github.com/tomtomwombat/fastbloom).

## Install

```bash
uv add pyfastbloom
```

## Usage

```python
from pyfastbloom import BloomFilter

f = BloomFilter(expected_items=1_000_000, false_positive_rate=0.001)
f.insert("42")
f.update(range(1000))
"42" in f
f.contains(b"bytes")

data = f.to_bytes()
g = BloomFilter.from_bytes(data)
g == f

a = BloomFilter(1000, seed=7)
b = BloomFilter(1000, seed=7)
a.union(b)
a.intersect(b)
a.clear()

f.num_bits, f.num_hashes, f.seed
f.expected_false_positive_rate(500_000)
```
