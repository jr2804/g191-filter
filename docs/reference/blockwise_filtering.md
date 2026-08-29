---
title: Blockwise (Streaming) Filtering
---

## Blockwise Filtering

For large or streaming audio inputs, `BlockwiseFilter` processes the signal in
fixed-size chunks while maintaining the filter's internal delay lines between
calls. This avoids loading the entire input into memory and keeps the filter
state consistent across chunk boundaries.

### Python API

```python
from g191_filter import BlockwiseFilter, filter_array
import numpy as np

# Create a blockwise filter for the "irs8khz" filter with 4096-sample chunks
bw = BlockwiseFilter("irs8khz", block_size=4096)

# Process chunks of a large signal
output_chunks = []
for chunk in np.array_split(large_signal, len(large_signal) // 4096):
    out = bw.process(chunk)
    output_chunks.append(out)

# Concatenate all chunks
result = np.concatenate(output_chunks)
```

### State Management

The filter state (delay lines and phase counters) is kept in Rust between
calls. You can snapshot and restore it for checkpointing or parallel
processing:

```python
bw = BlockwiseFilter("hq_down_2_to_1", block_size=8192)

# Save state after processing a portion
state = bw.state

# Later (or in another process): restore and continue
bw.state = state

# Reset to zero state
bw.reset()
```

The `state` property is both readable and writable (getter + setter), so
snapshotting is simply assignment. The `BlockwiseFilter` also offers
`process_all(input_array)` for one-shot chunked processing and `reset()` to
return to a fresh zero state.

#### State format

The `state` property returns a flat `numpy.ndarray` of `float64`. Its length
depends on the filter type:

| Filter type    | State length                       |
| -------------- | ---------------------------------- |
| FIR            | `block_size + 1`                   |
| IIR (parallel) | `block_size * 2 + 1`               |
| IIR (cascade)  | `block_size * 4 + 1`               |
| IIR (direct)   | `block_size * 2 + 1`               |

### Chunked one-shot filtering

`filter_array` also supports explicit chunking via the `block_size`
parameter. When provided, it routes through `BlockwiseFilter` internally:

```python
from g191_filter import filter_array
import numpy as np

# Process in 8192-sample chunks (streaming path)
result = filter_array("mod_irs16khz", input_array, block_size=8192)

# Without chunking (one-shot path, default)
result = filter_array("mod_irs16khz", input_array)
```

### CLI Usage

```bash
# Filter a WAV file in chunks (streaming)
uv run g191_filter filter --filter-id irs8khz --input input.wav --output output.wav --block-size 4096

# Filter with explicit chunk size and in-place overwrite
uv run g191_filter filter --filter-id hq_down_2_to_1 --input signal.wav --block-size 8192
```

### When to Use Blockwise Filtering

- **Large files**: Avoids allocating a single large buffer.
- **Streaming**: Process audio as it arrives from a microphone or network
  socket.
- **Checkpointing**: Save and restore filter state for fault tolerance or
  parallel processing pipelines.
- **Real-time**: Fixed-size chunks align with audio frame periods.

For small-to-medium signals, the one-shot `filter_array` or `filter_wave`
paths are simpler and equally fast.
