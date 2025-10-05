#ifndef STUFF
#define THREADS 512;
#define SIZE 64
#endif

constexpr int threads = THREADS;
constexpr int per_block = threads / 32;
constexpr int size = SIZE;
constexpr int per_thread = size / 32;

extern "C" __global__ void kernel(const int k, const float* input, float* output)
{
    const int entry = per_block * blockIdx.x + (threadIdx.x / 32);
    const int widx = threadIdx.x % 32;

    if (entry >= k)
        return;

    float elems[per_thread];

    elems[0] = input[widx + size * entry];
    float maximum = elems[0];

    #pragma unroll
    for (int i = 1; i < per_thread; i++) {
        elems[i] = input[widx + 32 * i + size * entry];
        maximum = max(maximum, elems[i]);
    }

    maximum = max(maximum, __shfl_xor_sync(0xffffffff, maximum, 16));
    maximum = max(maximum, __shfl_xor_sync(0xffffffff, maximum, 8));
    maximum = max(maximum, __shfl_xor_sync(0xffffffff, maximum, 4));
    maximum = max(maximum, __shfl_xor_sync(0xffffffff, maximum, 2));
    maximum = max(maximum, __shfl_xor_sync(0xffffffff, maximum, 1));

    float denom = 0.0F;

    #pragma unroll
    for (int i = 0; i < per_thread; i++) {
        elems[i] = expf(elems[i] - maximum);
        denom += elems[i];
    }

    denom += __shfl_xor_sync(0xffffffff, denom, 16);
    denom += __shfl_xor_sync(0xffffffff, denom, 8);
    denom += __shfl_xor_sync(0xffffffff, denom, 4);
    denom += __shfl_xor_sync(0xffffffff, denom, 2);
    denom += __shfl_xor_sync(0xffffffff, denom, 1);

    #pragma unroll
    for (int i = 0; i < per_thread; i++) {
        output[widx + 32 * i + size * entry] = elems[i] / denom;
    }
}
