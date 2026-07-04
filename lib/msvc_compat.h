// MSVC compatibility header - emulates GCC builtins
#pragma once

#ifndef _MSC_VER
#error "This header is only for MSVC compilation"
#endif

#include <intrin.h>
#include <malloc.h>

// __builtin_expect is a GCC builtin, not available on MSVC
#define __builtin_expect(x, y) (x)

// __attribute__ is a GCC extension, not available on MSVC
#define __attribute__(x)

// ---- __builtin_clz / __builtin_clzll -- count leading zeros ----
#pragma intrinsic(_BitScanReverse, _BitScanReverse64)

static inline int __builtin_clz(unsigned int x) {
    unsigned long index;
    _BitScanReverse(&index, x);
    return (int)(31 ^ index);
}

static inline int __builtin_clzll(unsigned long long x) {
    unsigned long index;
    _BitScanReverse64(&index, x);
    return (int)(63 ^ index);
}

// ---- __builtin_ctz / __builtin_ctzll -- count trailing zeros ----
#pragma intrinsic(_BitScanForward, _BitScanForward64)

static inline int __builtin_ctz(unsigned int x) {
    unsigned long index;
    _BitScanForward(&index, x);
    return (int)index;
}

static inline int __builtin_ctzll(unsigned long long x) {
    unsigned long index;
    _BitScanForward64(&index, x);
    return (int)index;
}
