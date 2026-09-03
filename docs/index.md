---
title: Overview
hide:
- feedback
---

--8<-- "README.md"

```mermaid mindmap
  root((ITU-T G.191<br/>FIR/IIR Filters))
    FIR
      Low-Pass
        lp1p5_48khz
        lp2p5_48khz
        lp3p0_48khz
        lp4p0_48khz
      High-Pass
        hp1p5_48khz
        hp2p5_48khz
      Band-Pass
        bp5k_16khz
        bp100_5k_16khz
        bp14k_32khz
        bp20k_48khz
    IIR
      Low-Pass
        iir_lp_48khz
        iir_lp_8khz
        iir_casc_lp_3_to_1
      High-Pass
        iir_hp_48khz
        iir_hp_8khz
        dir_dc_removal
      Resonant
        iir_res_48khz
  Specifications
    IRS 8 kHz
    IRS 16 kHz
    Modified IRS 16 kHz
    Modified IRS 48 kHz
    G.712 PCM
    Direct DC Removal
```

--8<-- "README.md"

---

## Documentation

- [API reference](reference/api.md)
- [Blockwise filtering](guides/blockwise_filtering.md)
- [Credits](credits.md)
- [License](license.md)
- [Contributing](contributing.md)
- [Code of Conduct](code_of_conduct.md)
