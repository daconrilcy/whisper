# Synthetic PE fixtures

These files are data for `ci/check-worker-imports.ps1 -SelfTest`. They are
minimal synthetic PE32+ headers and import tables, not runnable workers.
Never execute them. The self-test reads them directly from this versioned
directory; no compiled artifact or previous campaign is required.

| File | Imports / expected check |
| --- | --- |
| `no-cuda.bin` | KERNEL32.dll; CPU, Parakeet CPU and desktop accepted; GPU refused |
| `cublas.bin` | cublas64_12.dll; CPU, Parakeet CPU and desktop refused |
| `delay-cuda.bin` | delayed nvcuda.dll; CPU, Parakeet CPU and desktop refused |
| `gpu.bin` | cudart64_12.dll and nvcuda.dll; GPU accepted, CPU kinds refused |
| `cudart-only.bin` | cudart64_12.dll; GPU refused because nvcuda is absent |
| `nvcuda-only.bin` | nvcuda.dll; GPU refused because cudart is absent |
| `truncated-direct.bin` | declared table ends before the next import and terminator; parse refused |
| `truncated-delay.bin` | declared delayed table ends before the next import and terminator; parse refused |
| `non-pe.bin` | three arbitrary bytes; parse refused |

The PE fixtures contain a DOS signature at 0x00, PE signature at 0x80,
PE32+ optional header at 0x98, a section mapping RVA 0x1000 to file offset
0x400, import descriptors at 0x400 and DLL names at 0x440. The optional
real witness requires both its explicit path and SHA-256 and must be
rejected for CUDA imports by the CPU policy.
